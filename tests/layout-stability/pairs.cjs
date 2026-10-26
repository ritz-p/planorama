const input = 'tests/layout-stability/base.json';
const identity = (address, state = null, mode = 'managed', deposed = null) => ({state,address,mode,deposed});
const state = (name = null, patches = []) => ({name,input,patches});
const config = (patches = [], args = []) => ({states:[state(null,patches)],args});
const extra = [
  {op:'add',path:['resource_changes','-'],value:{address:'aws_instance.extra',type:'aws_instance',change:{actions:['create']}}},
  {op:'add',path:['configuration','root_module','resources','-'],value:{address:'aws_instance.extra',expressions:{subnet_id:{references:['aws_subnet.app.id']}}}}
];
const relationship = [{op:'add',path:['configuration','root_module','resources',3,'expressions','user_data'],value:{references:['aws_instance.app.id']}}];
const longLabel = [{op:'add',path:['resource_changes',2,'previous_address'],value:'module.'+'previous_module_'.repeat(8)+'.aws_instance.app'}];
const pair = (id, description, before, after, affected = [], intentional_restructure = false) => ({id,description,before,after,affected,intentional_restructure});
module.exports = {schema_version:1,pairs:[
  {...pair('identical','Identical input and settings control',config(),config()),control:true},
  pair('resource-add','Add one instance within the retained subnet',config(),config(extra),[identity('aws_instance.extra'),identity('aws_subnet.app')]),
  pair('resource-remove','Remove that instance',config(extra),config(),[identity('aws_instance.extra'),identity('aws_subnet.app')]),
  pair('relationship-add','Add one explicit dependency between retained instances',config(),config(relationship),[identity('aws_instance.app'),identity('aws_instance.other')]),
  pair('relationship-remove','Remove one explicit dependency',config(relationship),config(),[identity('aws_instance.app'),identity('aws_instance.other')]),
  pair('long-label','Lengthen a move-provenance label while retaining current identity',config(),config(longLabel),[identity('aws_instance.app')]),
  pair('address-format','Change qualified labels to Terraform labels',config(),config([],['--address-format','terraform']),[],true),
  pair('legend-offset','Omit the legend: intentional whole-diagram translation',config(relationship),config(relationship,['--omit-relationship-legend']),[],true),
  pair('selection','Focus on app; hidden resources are removals, not zero-sized retained frames',config(),config([],['--focus','aws_instance.app','--focus-depth','0']),[identity('aws_instance.app')],true),
  pair('logical-to-spatial','Change a resource presentation intentionally',config(),config([],['--spatial-type','aws_iam_role']),[identity('aws_iam_role.app')],true),
  pair('named-state-offset','Lengthen a label in the first state and measure the second relative to its own scope',
    {states:[state('alpha'),state('beta')],args:[]},
    {states:[state('alpha',longLabel),state('beta')],args:[]},
    [identity('aws_instance.app','alpha')])
]};
