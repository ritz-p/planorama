'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {extract,buildEvidence,render}=require('../../scripts/aws-semantic-audit.cjs');
const read=p=>JSON.parse(fs.readFileSync(p));
test('candidate extraction respects nested collections and sensitive boundaries',()=>{
 assert.deepEqual(extract({attributes:{role_arn:{optional:true,type:'string'},computed_id:{computed:true},secret_id:{optional:true,sensitive:true},write_id:{optional:true,write_only:true},secret:{sensitive:true,nested_type:{attributes:{target_id:{optional:true}}}}},block_types:{network:{block:{attributes:{subnets:{required:true,type:['set','string']},security_groups:{optional:true}}}}}}),['network.security_groups','network.subnets','role_arn']);
});
test('untrusted schema bytes cannot replace pinned evidence',()=>{
 assert.throws(()=>buildEvidence(Buffer.from('{}')),/SHA-256/);
});
test('register covers every exact managed type once and rejects incomplete evidence',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const evidence=read('docs/aws-semantic-evidence.json');
 const output=render(evidence);
 const names=[...output.matchAll(/^\| \[`(aws_[^`]+)`\]/gm)].map(m=>m[1]);
 assert.deepEqual(names.sort(),inventory.resources.map(r=>r.type).sort());
 assert.equal(output,fs.readFileSync('docs/aws-semantic-gaps.md','utf8'));
 delete evidence.candidates.aws_msk_cluster;
 assert.throws(()=>render(evidence),/inventory mismatch/);
});
test('representative evidence includes nested network paths without asserting implementation',()=>{
 const {candidates}=read('docs/aws-semantic-evidence.json');
 assert.ok(candidates.aws_msk_cluster.includes('broker_node_group_info.client_subnets'));
 assert.ok(candidates.aws_apprunner_service.includes('network_configuration.egress_configuration.vpc_connector_arn'));
 const decisions=read('docs/aws-semantic-decisions.json').types;
 assert.equal(decisions.aws_apprunner_service.status,'Basic-only');
 assert.equal(decisions.aws_dynamodb_table.status,'Partial');
 assert.match(decisions.aws_dynamodb_table.gap,/NOT implemented/);
});

test('NoSQL audit covers its exact managed denominator with six dimensions and pinned contracts',()=>{
 const services=new Set(['dynamodb','dax','elasticache','memorydb','keyspaces','qldb','timestreaminfluxdb','timestreamquery','timestreamwrite']);
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===718);
 assert.equal(expected.length,39);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review}] of entries) {
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.ok(review[key]?.trim().length>0,type+': '+key);
 }
});

test('streaming audit covers its exact managed denominator and preserves bounded evidence',()=>{
 const services=new Set('kafka kafkaconnect mq kinesis kinesisanalytics kinesisanalyticsv2 kinesisvideo firehose events scheduler pipes schemas sns sqs sfn swf mwaa'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===721);
 assert.equal(expected.length,55);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap}] of entries) {
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#721/);
 }
 const decisions=Object.fromEntries(entries);
 assert.deepEqual(decisions.aws_mq_broker.review.shapes.subnet_ids,[['set','string']]);
 assert.deepEqual(decisions.aws_msk_serverless_cluster.review.shapes['vpc_config.subnet_ids'],['list',['set','string']]);
 assert.match(decisions.aws_sns_topic_subscription.review.references,/SQS\/Lambda/);
 assert.match(decisions.aws_sqs_queue.review.summaries,/Supported:.*#601/);
});
