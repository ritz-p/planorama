import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
DIRECTORY = ROOT / 'examples/multi-account'
document = ET.parse(DIRECTORY / 'diagram.svg').getroot()
result = {'canvas': {k: document.get(k) for k in ['width', 'height']},
          'svg_bytes': (DIRECTORY / 'diagram.svg').stat().st_size,
          'html_bytes': (DIRECTORY / 'diagram.html').stat().st_size if (DIRECTORY / 'diagram.html').exists() else None,
          'states': {}}
for state in document.iter():
    name = state.get('data-state-id')
    if name is None:
        continue
    inputs = json.loads((DIRECTORY / (name + '.json')).read_text())
    summary = next((n.text for n in state.iter() if n.text and 'spatial cards' in n.text), '')
    paths = [n for n in state.iter() if n.tag.endswith('path') and n.get('data-edge-kind')]
    entries = [n for n in state.iter() if n.get('data-relationship-index')]
    indexes = [n for n in state.iter() if n.get('data-relationship-entry')]
    result['states'][name] = {
        'input': len(inputs['resource_changes']), 'summary': summary,
        'local_paths': len(paths), 'index_cards': len(entries),
        'indexed_relationships': len(indexes),
        'account_panels': sum(n.get('data-account-panel') == 'true' for n in state.iter()),
        'region_panels': sum(n.get('data-region') is not None for n in state.iter()),
        'global_panels': sum(n.get('data-global') == 'true' for n in state.iter()),
    }
cross = [n for n in document.iter() if n.get('data-cross-state') == 'true']
result['cross_paths'] = len(cross)
metadata = next(n for n in document.iter() if n.get('id') == 'cross-state-relationships')
result['cross_references'] = len(json.loads(metadata.text))
result['mappings'] = 3
print(json.dumps(result, indent=2))
