const fs=require('node:fs');
if(process.argv.length!==4)throw Error('Usage: node tests/support/verify-aws-semantic-contracts.cjs SCHEMA SOURCE');
require('../../scripts/aws-semantic-audit.cjs').buildEvidence(fs.readFileSync(process.argv[2]));
const schema=JSON.parse(fs.readFileSync(process.argv[2])).provider_schemas['registry.terraform.io/hashicorp/aws'].resource_schemas;
const text=fs.readFileSync('docs/aws-semantic-audit.md','utf8');
for(const line of text.split('\n').filter(l=>l.startsWith('| `aws_'))){
 const cells=line.split('|');const type=cells[1].match(/`([^`]+)`/)[1];
 const paths=[...cells[2].matchAll(/`([^`]+)`/g)].map(m=>m[1]);
 for(const path of paths){let block=schema[type].block;const parts=path.split('.');for(const part of parts.slice(0,-1)){block=block.block_types?.[part]?.block||block.attributes?.[part]?.nested_type;if(!block)throw Error(type+': '+path);}
 const attr=block.attributes?.[parts.at(-1)];if(!attr||!(attr.optional||attr.required))throw Error(type+': '+path);
 console.log(type+' '+path+' '+JSON.stringify(attr.type || attr.nested_type));}
 const inv=JSON.parse(fs.readFileSync('tests/fixtures/aws-inventory/6.70.0.json'));const resource=inv.resources.find(r=>r.type===type);
 const doc=fs.readFileSync(process.argv[3]+'/'+resource.documentation,'utf8');for(const p of paths)if(!doc.includes('`'+p.split('.').at(-1)+'`'))throw Error('Missing documented field '+type+' '+p);
}
