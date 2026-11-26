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
  return escape(source + ':' + (line + 1) + ':' + column + ': replace ' +
    operator + ' with ' + replacement + ' in ' + functionName);
});
console.log('^(?:' + patterns.join('|') + ')$');
