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
