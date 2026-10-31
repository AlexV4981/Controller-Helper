export const linkedActions=(bindings,physical)=>physical==='none'?[]:Object.keys(bindings).filter(a=>bindings[a]===physical);
export const followGame=(previous,next,busy)=>!busy&&next&&next!==previous?next:null;
export function diagramNodes(system,keys){
 const sideways=system==='wii-sideways';
 const map=system==='physical'?{up:[105,96],down:[105,164],left:[70,130],right:[140,130],south:[396,171],east:[430,137],west:[362,137],north:[396,103],lb:[105,49],rb:[413,49],lt:[155,49],rt:[363,49],back:[236,113],start:[284,113],guide:[260,68],ls:[200,190],rs:[320,190]}:sideways?{Up:[105,96],Down:[105,164],Left:[70,130],Right:[140,130],A:[240,128],B:[290,178],'1':[367,130],'2':[432,130],Select:[211,76],Home:[267,76],Start:[323,76]}:{Up:[103,103],Down:[103,171],Left:[69,137],Right:[137,137],A:[430,137],B:[396,171],X:[396,103],Y:[362,137],Start:[284,113],Select:[236,113],Home:[260,68],L:[105,49],R:[413,49],ZL:[155,49],ZR:[363,49],C:[202,178],Z:[318,178],'1':[364,205],'2':[430,205]};
 let extra=0;return keys.map(key=>{const pos=map[key]||[55+(extra%5)*103,250+Math.floor(extra++/5)*57];return {key,x:pos[0],y:pos[1]};});
}
export function renderDiagram(target,{system,keys,labels,bindings,physical,selectedAction,selectedPhysical,onSelect}){
 target.replaceChildren();const box=document.createElement('div');box.className='controller-diagram '+(system==='wii-sideways'?'remote-diagram':'pad-diagram');
 const nodes=diagramNodes(system,keys);box.style.height=Math.max(330,...nodes.map(n=>n.y+32))+'px';
 const body=document.createElement('div');body.className='diagram-body';body.setAttribute('aria-hidden','true');box.append(body);
 for(const {key,x,y} of nodes){const button=document.createElement('button');button.type='button';button.className='diagram-control';button.style.left=(x/520*100)+'%';button.style.top=y+'px';button.dataset.control=key;
 const bound=physical?linkedActions(bindings,key).length>0:bindings[key]&&bindings[key]!=='none';
 const selected=physical?key===(selectedPhysical||bindings[selectedAction]):selectedPhysical?bindings[key]===selectedPhysical:key===selectedAction;
 button.classList.toggle('bound',!!bound);button.classList.toggle('linked',!!selected);button.setAttribute('aria-pressed',String(!!selected));
 const short={up:'↑',down:'↓',left:'←',right:'→',back:'View',start:'Menu',guide:'⌂',ls:'L3',rs:'R3','lx-':'LX−','lx+':'LX+','ly-':'LY−','ly+':'LY+','rx-':'RX−','rx+':'RX+','ry-':'RY−','ry+':'RY+'};const emulated={Start:'+',Select:'−',Home:'⌂',Up:'↑',Down:'↓',Left:'←',Right:'→','Tilt left':'Tilt ←','Tilt right':'Tilt →','Tilt forward':'Tilt ↑','Tilt backward':'Tilt ↓'};button.textContent=physical?(short[key]||labels?.[key]||key):(emulated[key]||key);button.title=(physical?linkedActions(bindings,key).join(', ')||'Unassigned':bindings[key]||'Unassigned');button.setAttribute('aria-label',(physical?'Physical ':'Emulated ')+(labels?.[key]||key)+': '+button.title);button.addEventListener('click',()=>onSelect(key));box.append(button);}
 target.append(box);
}

export const appliedSave=(state,revision)=>Number.isSafeInteger(revision)&&revision>0&&state.applied_revision>=revision&&state.desired_revision>=revision&&state.apply_phase!=='error'&&!state.pending;

export function sessionSummary(state){
 const active=state.games?.find(g=>g.id===state.runtime?.game);
 if(active)return {title:active.title,detail:state.runtime?.message||'Game identified from its emulator window.'};
 const running=state.running||[];
 if(running.some(name=>/melonds/i.test(name)))return {
  title:'melonDS open · game not exposed',
  detail:'The loaded DS game is not identified. Stock melonDS needs the optional identity patch; with that patch installed, refresh the DS library to match its content ID. You can keep playing directly with its saved controls. Choose Shared console presets → Nintendo DS to edit direct-use controls, or select a game to edit its optional isolated layout.'
 };
 if(running.length)return {title:'Emulator open · game not identified',detail:state.runtime?.message||'Choose a game to edit; detection is unavailable.'};
 return {title:state.last_game?'Last played · '+(state.games?.find(g=>g.id===state.last_game)?.title||state.last_game):'No game running',detail:state.runtime?.message||''};
}
export function scopeNote(editing,custom){
 if(!editing)return 'Editing shared console controls. Direct emulator use keeps these controls; games with a custom layout keep their own controls.';
 return `Editing only ${editing.title}. ${custom?'Custom game layout.':'Inherits shared controls until you save.'} ${editing.system==='ds'?'An optional Studio launch uses this isolated layout. Direct melonDS use keeps its shared controls.':'Dolphin selects this layout automatically.'}`;
}
export function saveMessage(system,game){
 return 'Your saved bindings have been applied.'+(system==='ds'&&game?' This isolated DS layout is used by an optional Studio launch; direct melonDS use keeps its shared controls.':'');
}
