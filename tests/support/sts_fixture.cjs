const http = require('node:http');
const { writeFileSync } = require('node:fs');

const body = Buffer.from(`<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleResult><Credentials><AccessKeyId>ASIAFIXTUREONLY000000</AccessKeyId>
<SecretAccessKey>fixture-only-secret</SecretAccessKey><SessionToken>fixture-only-token</SessionToken>
<Expiration>2099-01-01T00:00:00Z</Expiration></Credentials>
<AssumedRoleUser><AssumedRoleId>fixture</AssumedRoleId>
<Arn>arn:aws:sts::123456789012:assumed-role/Fixture/test</Arn></AssumedRoleUser>
</AssumeRoleResult><ResponseMetadata><RequestId>fixture</RequestId></ResponseMetadata></AssumeRoleResponse>`);

function start(portFile) {
  const server = http.createServer((request, response) => {
    request.resume();
    request.on('end', () => {
      if (request.method !== 'POST') {
        response.writeHead(501);
        response.end();
        return;
      }
      response.writeHead(200, { 'Content-Type': 'text/xml', 'Content-Length': body.length });
      response.end(body);
    });
  });
  server.on('error', error => { console.error(error); process.exitCode = 1; });
  server.listen(0, '127.0.0.1', () => {
    try { writeFileSync(portFile, String(server.address().port)); }
    catch (error) { console.error(error); process.exitCode = 1; server.close(); }
  });
  for (const signal of ['SIGTERM', 'SIGINT']) process.once(signal, () => {
    server.close();
    server.closeAllConnections();
  });
  return server;
}

if (require.main === module) {
  if (process.argv.length !== 3) throw new Error('Usage: node sts_fixture.cjs PORT_FILE');
  start(process.argv[2]);
}
module.exports = { start };
