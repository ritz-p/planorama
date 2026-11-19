'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const [directory, credentials] = process.argv.slice(2);
const entries = fs.readFileSync(credentials, 'utf8').split(/\r?\n/);
const secrets = entries.flatMap(line => {
  const match = /^\s*aws_(access_key_id|secret_access_key|session_token)\s*=\s*(\S+)\s*$/i.exec(line);
  return match ? [match[2]] : [];
});
if (secrets.length < 2) throw new Error('Emulator credential file is incomplete');
for (const name of fs.readdirSync(directory)) {
  const file = path.join(directory, name);
  const data = name.endsWith('.tfplan') ? execFileSync('unzip', ['-p', file]) : fs.readFileSync(file);
  if (secrets.some(secret => data.includes(secret)) || data.includes('PRIVATE KEY-----')) {
    throw new Error('Credential material in ' + name);
  }
}
