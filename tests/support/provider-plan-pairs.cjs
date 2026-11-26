const versions = ['1.9.8', '1.10.5', '1.11.4', '1.12.2', '1.13.5', '1.14.9', '1.15.9', '1.16.6'];
const pairs = versions.flatMap(core => [['5.100.0', 'aws5'], ['6.0.0', 'aws6'], ['6.70.0', 'aws670']].map(([provider, fixture]) => [core, provider, fixture]));

module.exports = {versions, pairs};
