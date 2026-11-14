const fs = require('node:fs');
const inventory = JSON.parse(fs.readFileSync('tests/fixtures/cloudflare-inventory/5.28.1.json'));
const rows = ['use crate::model::EntityMode;', '', '#[rustfmt::skip]', 'pub(super) const TYPES: &[(&str, EntityMode, &[&str], &[&str])] = &['];
for (const r of inventory.resources) {
  const a = r.attributes;
  const account = r.type === 'cloudflare_account' && r.mode === 'managed' ? ['id'] : a.account?.nested_type?.attributes?.id?.type === 'string' ? ['account', 'id'] : a.account_id?.type === 'string' ? ['account_id'] : [];
  const zone = r.type === 'cloudflare_zone' && r.mode === 'managed' ? ['id'] : a.zone_id?.type === 'string' ? ['zone_id'] : [];
  rows.push(`    ("${r.type}", EntityMode::${r.mode === 'managed' ? 'Managed' : 'Data'}, &${JSON.stringify(account)}, &${JSON.stringify(zone)}),`);
}
rows.push('];', '');
fs.writeFileSync('src/provider/cloudflare/types.rs', rows.join('\n'));
