function generationInformation(byId) {
  const panel = byId('generation-details');
  const metadata = JSON.parse(byId('viewer-generation').textContent);
  const paragraph = text => {
    const p = document.createElement('p'); p.textContent = text; panel.append(p);
  };
  if (!metadata || metadata.schema_version !== 1) {
    paragraph('Generation metadata unavailable. Selection and generation settings cannot be established from this diagram.');
    return;
  }
  paragraph(`${metadata.generator.name} ${metadata.generator.version}. Revision: ${metadata.generator.revision || 'unavailable'}.`);
  paragraph(metadata.selected_view ? 'Selected view. Omitted resources may still exist in Terraform.' : 'Full input view: no changed-only, module or focus selection. This describes the supplied inputs, not all resources in Terraform.');
  paragraph('Values below are effective generation settings. Whether a value was explicitly supplied or defaulted is unavailable; “matches default” describes the value only.');
  const defaults = { changed_only: false, modules: [], focus: null, focus_depth: null, spatial_types: [], address_format: 'qualified', icon_mode: 'service-name', relationship_legend: 'full', html_diagnostics: false, html_semantic_diagnostics: false, states: [], remote_state_mappings: [] };
  const labels = { changed_only: 'Changed only', modules: 'Module selection', focus: 'Focus', focus_depth: 'Focus depth', spatial_types: 'Spatial type overrides', address_format: 'Address format', icon_mode: 'Service labels', relationship_legend: 'Relationship legend', html_diagnostics: 'Routing diagnostics', html_semantic_diagnostics: 'Semantic diagnostics', states: 'Named states', remote_state_mappings: 'Remote state mappings' };
  const list = document.createElement('dl');
  for (const [key, label] of Object.entries(labels)) {
    const term = document.createElement('dt'), value = document.createElement('dd');
    term.textContent = label;
    const setting = metadata.settings[key];
    const display = setting === null ? 'disabled / not applicable' : typeof setting === 'boolean' ? (setting ? 'enabled' : 'disabled') : Array.isArray(setting) && !setting.length ? 'none' : typeof setting === 'string' ? setting : JSON.stringify(setting);
    value.textContent = setting === undefined ? 'unavailable' : `${display}${JSON.stringify(setting) === JSON.stringify(defaults[key]) ? ' (matches default)' : ''}`;
    list.append(term, value);
  }
  panel.append(list);
}
