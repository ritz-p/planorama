function vpcFolding(context, index, coordinator, renderResults) {
  const {svg, byId} = context;
  const menu = byId('vpc-destination'), toggle = byId('vpc-toggle');
  const frames = [...svg.querySelectorAll('[data-container], [data-route-table], [data-member-cluster]')];
  const vpcs = frames.map(el => index.targets.get(el.dataset.container)).filter(r => r?.type === 'aws_vpc');
  if (!vpcs.length) return;
  byId('vpc-controls').hidden = false;
  for (const r of vpcs) menu.add(new Option(r.state ? r.state + ': ' + r.address : r.address, r.id));
  const closed = new Set();
  const native = new Map(index.resources.map(r => [JSON.stringify([r.state, r.identity]), r.id]));
  for (const el of index.components) native.set(JSON.stringify([el.closest('[data-state-id]')?.dataset.stateId || '', el.dataset.architectureId]), el.id);
  const relations = [];
  for (const data of svg.querySelectorAll('metadata[id$="architecture-relationships"]')) {
    const state = data.closest('[data-state-id]')?.dataset.stateId || '';
    for (const r of JSON.parse(data.textContent)) {
      const from = native.get(JSON.stringify([state, r.from])), to = native.get(JSON.stringify([state, r.to]));
      if (from && to) relations.push({from, to, kind:r.kind, direction:r.directionality});
    }
  }
  for (const path of index.edges.filter(el => el.dataset.crossState)) {
    relations.push({from:path.dataset.source, to:path.dataset.target, kind:'dependency', direction:'directed'});
  }
  function element(parent, name, attributes, text) {
    const el = document.createElementNS(svg.namespaceURI, name);
    for (const [key,value] of Object.entries(attributes)) el.setAttribute(key,value);
    if (text !== undefined) el.textContent = text;
    parent.append(el);
    return el;
  }
  function refreshButton() {
    toggle.textContent = closed.has(menu.value) ? 'Expand VPC' : 'Collapse VPC';
    toggle.setAttribute('aria-expanded', String(!closed.has(menu.value)));
  }
  function redraw() {
    for (const el of svg.querySelectorAll('[data-vpc-hidden]')) el.removeAttribute('data-vpc-hidden');
    for (const el of svg.querySelectorAll('[data-vpc-overlay]')) el.remove();
    if (!closed.size) return;
    const representative = new Map();
    for (const r of index.resources) {
      let id = r.id;
      const seen = new Set();
      while (!closed.has(id) && !seen.has(id)) {
        seen.add(id);
        id = index.targets.get(id)?.context?.[0];
        if (!id) break;
      }
      if (closed.has(id)) representative.set(r.id,id);
    }
    const hide = el => el?.setAttribute('data-vpc-hidden','');
    for (const [id,owner] of representative) {
      const el = document.getElementById(id);
      if (id !== owner) { hide(el); continue; }
      const [x,y] = el.dataset.presentationHeader.split(' ').map(Number);
      for (const child of el.children) if (child.localName !== 'title') hide(child);
      const card = element(el,'g',{'data-vpc-overlay':''});
      const name = index.targets.get(id).address;
      const label = element(card,'text',{x:x+12,y:y+28,'font-size':15,fill:'#134e4a'},name);
      const width = Math.max(140,label.getComputedTextLength()+24);
      const rect = element(card,'rect',{x,y,width,height:44,rx:8,fill:'#ecfdf5',stroke:'#047857','stroke-width':2});
      card.prepend(rect);
    }
    for (const frame of frames) if (representative.has(frame.dataset.container || frame.dataset.routeTable || frame.dataset.memberCluster)) hide(frame);
    for (const mask of svg.querySelectorAll('[data-occluder]')) {
      if (representative.has(mask.dataset.occluder)) hide(mask);
    }
    const affected = new Set();
    for (const edge of svg.querySelectorAll('[data-source][data-target]')) {
      if (representative.has(edge.dataset.source) || representative.has(edge.dataset.target)) {
        hide(edge);
        for (const id of (edge.dataset.relationshipIds || '').split(' ').filter(Boolean)) affected.add(id);
        if (edge.dataset.crossState && edge.nextElementSibling?.localName === 'circle') hide(edge.nextElementSibling);
      }
    }
    for (const marker of svg.querySelectorAll('[data-relationship-marker]')) {
      if (affected.has(marker.querySelector('a')?.getAttribute('href')?.slice(1))) hide(marker);
    }
    for (const id of affected) hide(document.getElementById(id));
    for (const dot of svg.querySelectorAll('[data-junction-edges]')) {
      const prefix = dot.closest('[data-state-id]')?.id.replace(/state$/, '') || '';
      if (JSON.parse(dot.dataset.junctionEdges).some(pair => pair.some(id => representative.has(prefix+id)))) hide(dot);
    }
    const groups = new Map();
    for (const {from,to,kind,direction} of relations) {
      if (!representative.has(from) && !representative.has(to)) continue;
      let a = representative.get(from) || from, b = representative.get(to) || to;
      if (a === b || !byId(a) || !byId(b)) continue;
      if (direction === 'undirected' && a > b) [a,b] = [b,a];
      const key = JSON.stringify([a,b,kind,direction]);
      const group = groups.get(key) || {a,b,kind,direction,count:0};
      group.count++;
      groups.set(key,group);
    }
    const overlay = element(svg,'g',{'data-vpc-overlay':''});
    const rootMatrix = svg.getScreenCTM().inverse();
    const center = id => {
      const el = byId(id), box = el.getBBox();
      const p = new DOMPoint(box.x+box.width/2, box.y+box.height/2).matrixTransform(rootMatrix.multiply(el.getScreenCTM()));
      return {x:p.x,y:p.y,w:box.width/2,h:box.height/2};
    };
    const counts = new Map();
    for (const {a,b,kind,direction,count} of groups.values()) {
      const p = center(a), q = center(b), dx=q.x-p.x, dy=q.y-p.y;
      const length=Math.hypot(dx,dy) || 1;
      const pair=JSON.stringify([a,b].sort()), n=counts.get(pair) || 0;
      counts.set(pair,n+1);
      const offset=(a < b ? 1 : -1)*Math.ceil(n/2)*(n%2 ? 52 : -52);
      const mid={x:(p.x+q.x)/2-dy/length*offset,y:(p.y+q.y)/2+dx/length*offset};
      const port = box => {
        const x=mid.x-box.x, y=mid.y-box.y;
        const scale=Math.min(box.w/Math.max(Math.abs(x),0.001),box.h/Math.max(Math.abs(y),0.001),1);
        return [box.x+x*scale,box.y+y*scale].join(' ');
      };
      const line=element(overlay,'g',{'data-vpc-aggregate':kind,'data-directionality':direction,'data-count':count,'data-from':a,'data-to':b});
      const path=element(line,'path',{d:'M '+port(p)+' Q '+mid.x+' '+mid.y+' '+port(q),fill:'none',stroke:'#334155','stroke-width':2});
      if (direction === 'directed') {
        const marker=byId(a).closest('svg').querySelector('marker[id$="arrow"]');
        if (marker) path.setAttribute('marker-end','url(#'+marker.id+')');
      }
      const name = id => index.targets.get(id)?.address || index.title(byId(id));
      element(line,'title',{},name(a)+(direction === 'undirected' ? ' ↔ ' : ' → ')+name(b)+' · '+kind+' × '+count);
      element(line,'text',{x:mid.x,y:mid.y-4,'text-anchor':'middle','font-size':14,fill:'#0f172a',stroke:'white','stroke-width':4,'paint-order':'stroke'},kind+' × '+count);
    }
  }
  menu.addEventListener('change',refreshButton);
  toggle.addEventListener('click',() => {
    coordinator.clear();
    if (closed.has(menu.value)) closed.delete(menu.value); else closed.add(menu.value);
    redraw(); refreshButton(); renderResults();
  });
  refreshButton();
  return redraw;
}
