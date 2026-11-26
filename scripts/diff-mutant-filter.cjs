const {readFileSync} = require('node:fs');

const source = 'src/diff.rs';
const lines = readFileSync(source, 'utf8').split('\n');
const changes = [
  ['counts[new] == 1', '==', '!='],
  ['a.provider == b.provider', '==', '!='],
  ['a.mode == b.mode', '==', '!='],
  ['!uncertain.contains(&e.from) && !uncertain.contains(&e.to)', '&&', '||'],
];

const escape = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const patterns = changes.map(([expression, operator, replacement]) => {
  const line = lines.findIndex(text => text.includes(expression));
  if (line < 0 || lines.slice(line + 1).some(text => text.includes(expression))) {
    throw Error('Expected one diff matching predicate: ' + expression);
  }
  const column = lines[line].indexOf(expression) + expression.indexOf(operator) + 1;
  return escape(source + ':' + (line + 1) + ':' + column + ': replace ' +
    operator + ' with ' + replacement + ' in compare');
});
console.log('^(?:' + patterns.join('|') + ')$');
