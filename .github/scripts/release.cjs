const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');

const targets = [
  'x86_64-unknown-linux-gnu',
  'x86_64-pc-windows-msvc',
  'aarch64-apple-darwin',
  'x86_64-apple-darwin',
];
const run = (command, args) => execFileSync(command, args, {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
const sha256 = data => crypto.createHash('sha256').update(data).digest('hex');
const archiveName = (version, target) => `planorama-v${version}-${target}.tar.gz`;

function validateVersion(version, tag) {
  if (!/^\d+\.\d+\.\d+$/.test(version) || tag !== `v${version}`) {
    throw new Error(`Release tag ${tag} must equal stable package version v${version}`);
  }
}

function validateRequest(context, version, publishRequested = false) {
  if (context.eventName !== 'workflow_dispatch') throw new Error('Release builds require workflow_dispatch');
  if (typeof publishRequested !== 'boolean') throw new Error('Publication request must be a boolean');
  const tag = context.ref.startsWith('refs/tags/') ? context.ref.slice('refs/tags/'.length) : '';
  if (publishRequested && !tag) throw new Error('Publication requires an existing version tag');
  validateVersion(version, tag || `v${version}`);
}

function smoke(binary, version, directory) {
  const revision = run('git', ['-c', `safe.directory=${process.cwd()}`, 'rev-parse', 'HEAD']).trim();
  if (run(binary, ['--version']).trim() !== `planorama ${version} (revision: ${revision})`) throw new Error('Wrong binary version or revision');
  if (!run(binary, ['--help']).includes('Usage: planorama')) throw new Error('Missing help');
  for (const format of ['svg', 'html']) {
    const output = path.join(directory, `smoke.${format}`);
    run(binary, ['examples/plan.json', '--format', format, '-o', output]);
    const content = fs.readFileSync(output, 'utf8');
    if (!content.includes('<svg ') || !content.includes('</svg>') || !content.includes('aws_vpc')) {
      throw new Error(`Invalid ${format} diagram`);
    }
    if (format === 'html' && (!/<html[\s>]/i.test(content) || !content.includes('</html>') || !content.includes('<script'))) {
      throw new Error('Invalid HTML viewer');
    }
  }
}

function notices(packages, destination) {
  const index = ['Third-party Cargo packages (including build/dev and other-target dependencies).', 'License expressions and original license/notice files follow.'];
  for (const pkg of packages.filter(pkg => pkg.source).sort((a, b) => a.id.localeCompare(b.id))) {
    const root = path.dirname(pkg.manifest_path);
    const files = fs.readdirSync(root).filter(name => /^(licen[cs]e|copying|notice|copyright|authors)([._-]|$)/i.test(name) && fs.statSync(path.join(root, name)).isFile());
    if (pkg.license_file && !files.includes(pkg.license_file)) files.push(pkg.license_file);
    index.push(`\n${pkg.name} ${pkg.version}\n${pkg.source}\nLicense: ${pkg.license || 'see license file'}`);
    if (!files.length) {
      if (pkg.name !== 'pratt' || pkg.version !== '0.4.0' || pkg.license !== 'MIT') throw new Error(`Missing license text: ${pkg.name}`);
      index.push(fs.readFileSync('third-party/pratt-0.4.0.txt', 'utf8'));
    }
    for (const file of files.sort()) index.push(`--- ${file} ---\n${fs.readFileSync(path.join(root, file), 'utf8')}`);
  }
  fs.writeFileSync(path.join(destination, 'THIRD-PARTY-NOTICES.txt'), `${index.join('\n')}\n`);

}

function pack(target) {
  if (!targets.includes(target)) throw new Error('Unsupported release target');
  const metadata = JSON.parse(run('cargo', ['metadata', '--locked', '--format-version', '1']));
  const version = metadata.packages.find(pkg => pkg.name === 'planorama' && !pkg.source).version;
  validateVersion(version, process.env.RELEASE_TAG || `v${version}`);
  const stem = archiveName(version, target).replace(/\.tar\.gz$/, '');
  const staging = path.resolve('target/release-pack', target);
  fs.rmSync(staging, {recursive: true, force: true});
  const directory = path.join(staging, stem);
  fs.mkdirSync(directory, {recursive: true});
  const executable = target.includes('windows') ? 'planorama.exe' : 'planorama';
  fs.copyFileSync(path.join(metadata.target_directory, target, 'release', executable), path.join(directory, executable));
  fs.chmodSync(path.join(directory, executable), 0o755);
  for (const file of ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE']) {
    fs.copyFileSync(file, path.join(directory, file));
  }
  fs.copyFileSync('docs/release-usage.txt', path.join(directory, 'README.txt'));
  notices(metadata.packages, directory);
  const rustDocs = path.join(run('rustc', ['+1.85.1', '--print', 'sysroot']).trim(), 'share/doc/rust');
  for (const file of ['COPYRIGHT', 'LICENSE-APACHE', 'LICENSE-MIT']) {
    fs.copyFileSync(path.join(rustDocs, file), path.join(directory, `RUST-${file}.txt`));
  }
  fs.mkdirSync('output/release', {recursive: true});
  const archive = path.resolve('output/release', archiveName(version, target));
  run('tar', ['-czf', archive, '-C', staging, stem]);
  const extracted = path.join(staging, 'extracted');
  fs.mkdirSync(extracted);
  run('tar', ['-xzf', archive, '-C', extracted]);
  for (const file of fs.readdirSync(directory)) {
    if (!fs.readFileSync(path.join(directory, file)).equals(fs.readFileSync(path.join(extracted, stem, file)))) {
      throw new Error(`Archive changed ${file}`);
    }
  }
  smoke(path.join(extracted, stem, executable), version, extracted);
  fs.writeFileSync(`${archive}.sha256`, `${sha256(fs.readFileSync(archive))}  ${path.basename(archive)}\n`);
}

function collect(directory, version) {
  const names = targets.map(target => archiveName(version, target)).sort();
  const expected = names.flatMap(name => [name, `${name}.sha256`]).sort();
  if (JSON.stringify(fs.readdirSync(directory).sort()) !== JSON.stringify(expected)) throw new Error('Incomplete or unexpected target assets');
  const checksums = names.map(name => {
    const line = `${sha256(fs.readFileSync(path.join(directory, name)))}  ${name}\n`;
    if (fs.readFileSync(path.join(directory, `${name}.sha256`), 'utf8') !== line) throw new Error(`Checksum mismatch: ${name}`);
    return line;
  }).join('');
  return [...names.map(name => ({name, data: fs.readFileSync(path.join(directory, name))})), {name: 'SHA256SUMS', data: Buffer.from(checksums)}];
}

async function publish({github, context, directory, version, tag, publishRequested = false}) {
  validateRequest(context, version, publishRequested);
  validateVersion(version, tag);
  if (!publishRequested || context.ref !== `refs/tags/${tag}`) throw new Error('Publication requires explicit workflow_dispatch on the matching tag');
  const assets = collect(directory, version);
  const repo = context.repo;
  const existing = await github.paginate(github.rest.repos.listReleases, {...repo, per_page: 100});
  if (existing.some(release => release.tag_name === tag)) throw new Error('Release already exists; no assets were overwritten');
  const {data: release} = await github.rest.repos.createRelease({...repo, tag_name: tag, target_commitish: context.sha, name: `Planorama ${tag}`, draft: true, body: 'Native CLI archives. See README.txt inside each archive. SHA256SUMS detects corruption; binaries are not signed or notarized.'});
  for (const asset of assets) {
    const {data: uploaded} = await github.rest.repos.uploadReleaseAsset({...repo, release_id: release.id, name: asset.name, data: asset.data, headers: {'content-type': 'application/octet-stream', 'content-length': asset.data.length}});
    if (uploaded.size !== asset.data.length || uploaded.digest !== `sha256:${sha256(asset.data)}`) throw new Error(`Upload verification failed: ${asset.name}`);
  }
  const uploaded = await github.paginate(github.rest.repos.listReleaseAssets, {...repo, release_id: release.id, per_page: 100});
  if (JSON.stringify(uploaded.map(asset => asset.name).sort()) !== JSON.stringify(assets.map(asset => asset.name).sort())) throw new Error('Unexpected uploaded assets');
  await github.rest.repos.updateRelease({...repo, release_id: release.id, draft: false});
}

module.exports = {targets, archiveName, validateVersion, validateRequest, collect, publish, notices};
if (require.main === module) pack(process.argv[2]);
