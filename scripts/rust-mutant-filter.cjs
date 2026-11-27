const {readFileSync} = require('node:fs');

const pilots = {
  diff: {
    source: 'src/diff.rs',
    functionName: 'compare',
    changes: [
      ['counts[new] == 1', '==', '!='],
      ['a.provider == b.provider', '==', '!='],
      ['a.mode == b.mode', '==', '!='],
      ['!uncertain.contains(&e.from) && !uncertain.contains(&e.to)', '&&', '||'],
    ],
  },
  'remote-output': {
    source: 'src/semantic/cross_state.rs',
    functionName: 'resolve_step',
    changes: [['!output.complete || output.sources.is_empty()', '||', '&&']],
  },
  'service-scope': {
    source: 'src/provider/aws/eventbridge.rs',
    functionName: 'bindings',
    changes: [['role || consumer_region == endpoint_region', '||', '&&']],
  },
  'saved-provider': {
    source: 'src/plan/evidence.rs',
    functionName: 'Evidence::apply',
    changes: [['node.deposed_key.is_some() || !node.provider.is_aws()', '||', '&&']],
  },
  'drift-identity': {
    source: 'src/plan/drift.rs',
    functionName: 'collect',
    changes: [['node.address == address && node.deposed_key == deposed_key', '&&', '||']],
  },
  'cached-data': {
    source: 'src/plan/collections.rs',
    functionName: 'annotate',
    changes: [['(node.mode == crate::model::EntityMode::Data)', '==', '!=']],
  },
  'memory-headroom': {
    source: 'src/generation/memory.rs',
    functionName: 'fits',
    changes: [['bytes <= limit', '<=', '>']],
  },
  'json-budget': {
    source: 'src/generation/budget.rs',
    functionName: "<impl DeserializeSeed<'de> for Seed<'_>>::deserialize",
    changes: [['self.depth >= 64 || self.remaining.get() == 0', '||', '&&']],
  },
  'named-limits': {
    source: 'src/named_inputs.rs',
    functionName: 'validate',
    changes: [['inputs.len() > 64', '>', '>=']],
  },
  'pseudonym-collision': {
    source: 'src/view/anonymize.rs',
    functionName: 'Projection::name',
    changes: [['name == value || self.reserved.contains(&name)', '||', '&&']],
  },
  'module-instance': {
    source: 'src/model/modules.rs',
    functionName: 'Address::contains',
    changes: [['name == n && (index.is_none() || index == i)', '&&', '||']],
  },
  'focus-depth': {
    source: 'src/view/filter.rs',
    functionName: 'apply',
    changes: [['!selected.contains(*k)', '!', null]],
  },
  'changed-context': {
    source: 'src/view/filter.rs',
    functionName: 'apply',
    changes: [['&& placement_available(&all.states[&child.0], child.1, |source|', '&&', '||']],
  },
  'coverage-scope': {
    source: 'src/coverage.rs',
    functionName: 'Coverage::collect',
    changes: [['node.provider.is_aws() && node.mode == EntityMode::Managed', '&&', '||']],
  },
  'check-target': {
    source: 'src/plan/checks.rs',
    functionName: 'resource',
    changes: [['|| address["type"].as_str()? != node.resource_type', '||', '&&']],
  },
  'plan-action': {
    source: 'src/plan.rs',
    functionName: 'parse_action',
    changes: [['actions.contains(&"create") && actions.contains(&"delete")', '&&', '||']],
  },
  'sensitive-path': {
    source: 'src/plan/changed_paths.rs',
    functionName: 'Collector::walk',
    changes: [['sensitive(&before_sensitive[key]) || sensitive(&after_sensitive[key])', '||', '&&']],
  },
  'input-file': {
    source: 'src/cli/files.rs',
    functionName: 'reject_same_file',
    changes: [['input_handle == output_handle', '==', '!=']],
  },
};
const {source, functionName, changes} = pilots[process.argv[2]];
const lines = readFileSync(source, 'utf8').split('\n');

const escape = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const patterns = changes.map(([expression, operator, replacement]) => {
  const line = lines.findIndex(text => text.includes(expression));
  if (line < 0 || lines.slice(line + 1).some(text => text.includes(expression))) {
    throw Error('Expected one matching predicate in ' + source + ': ' + expression);
  }
  const column = lines[line].indexOf(expression) + expression.indexOf(operator) + 1;
  const change = replacement === null ? 'delete ' + operator : 'replace ' + operator + ' with ' + replacement;
  return escape(source + ':' + (line + 1) + ':' + column + ': ' + change + ' in ' + functionName);
});
console.log('^(?:' + patterns.join('|') + ')$');
