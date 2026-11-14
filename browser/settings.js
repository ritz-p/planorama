const PlanoramaSettings = (() => {
  const maxBytes = 64 * 1024;
  const controls = Object.freeze({
    format: 'format', changed_only: 'changed', address_format: 'address-format',
    icon_mode: 'icon-mode', hide_relationship_legend: 'hide-legend',
    omit_relationship_legend: 'omit-legend', html_diagnostics: 'html-diagnostics',
    html_semantic_diagnostics: 'html-semantic-diagnostics',
  });
  const defaults = () => ({
    format: 'svg', changed_only: false, address_format: 'qualified', icon_mode: 'service-name',
    hide_relationship_legend: false, omit_relationship_legend: false,
    html_diagnostics: false, html_semantic_diagnostics: false, modules: [], spatial_types: [],
  });
  const enums = {format: ['svg', 'html'], address_format: ['qualified', 'terraform'], icon_mode: ['service-name']};
  const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
  const fail = () => { throw new Error('Unsupported or invalid browser settings.'); };
  function validate(value, spatialTypes) {
    if (!object(value)) fail();
    const initial = defaults();
    for (const key of Object.keys(value)) {
      if (!Object.hasOwn(initial, key) && key !== 'focus' && key !== 'focus_depth') fail();
    }
    const result = {...initial, ...value};
    for (const key of Object.keys(controls)) {
      if (Object.hasOwn(enums, key) ? !enums[key].includes(result[key]) : typeof result[key] !== 'boolean') fail();
    }
    for (const key of ['modules', 'spatial_types']) {
      if (!Array.isArray(result[key]) || result[key].length > 256 || !result[key].every(item => typeof item === 'string' && item !== '')) fail();
      result[key] = [...result[key]];
    }
    if (result.spatial_types.some(type => !spatialTypes.includes(type)) || new Set(result.spatial_types).size !== result.spatial_types.length) fail();
    if (result.focus === null) delete result.focus;
    if (result.focus_depth === null) delete result.focus_depth;
    if (Object.hasOwn(result, 'focus') && (typeof result.focus !== 'string' || result.focus === '')) fail();
    if (Object.hasOwn(result, 'focus_depth') && (!Object.hasOwn(result, 'focus') || !Number.isInteger(result.focus_depth) || result.focus_depth < 0 || result.focus_depth > 4294967295)) fail();
    if (result.format !== 'html' && (result.html_diagnostics || result.html_semantic_diagnostics)) fail();
    if (new TextEncoder().encode(JSON.stringify(result)).length > maxBytes) fail();
    return result;
  }
  function parse(text, spatialTypes) {
    if (new TextEncoder().encode(text).length > maxBytes) fail();
    let file;
    try { file = JSON.parse(text); } catch { fail(); }
    if (!object(file) || file.schema_version !== 1 || Object.keys(file).some(key => !['schema_version', 'settings'].includes(key))) fail();
    return validate(file.settings, spatialTypes);
  }
  function stringify(settings, spatialTypes) {
    const text = JSON.stringify({schema_version: 1, settings: validate(settings, spatialTypes)}, null, 2) + '\n';
    if (new TextEncoder().encode(text).length > maxBytes) fail();
    return text;
  }
  return Object.freeze({maxBytes, controls, defaults, validate, parse, stringify});
})();
if (typeof module !== 'undefined' && module.exports) module.exports = PlanoramaSettings;
