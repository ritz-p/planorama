import http.server
import pathlib
import sys


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        body = b'''<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleResult><Credentials><AccessKeyId>ASIAFIXTUREONLY000000</AccessKeyId>
<SecretAccessKey>fixture-only-secret</SecretAccessKey><SessionToken>fixture-only-token</SessionToken>
<Expiration>2099-01-01T00:00:00Z</Expiration></Credentials>
<AssumedRoleUser><AssumedRoleId>fixture</AssumedRoleId>
<Arn>arn:aws:sts::123456789012:assumed-role/Fixture/test</Arn></AssumedRoleUser>
</AssumeRoleResult><ResponseMetadata><RequestId>fixture</RequestId></ResponseMetadata></AssumeRoleResponse>'''
        self.send_response(200)
        self.send_header("Content-Type", "text/xml")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
pathlib.Path(sys.argv[1]).write_text(str(server.server_port))
server.serve_forever()
