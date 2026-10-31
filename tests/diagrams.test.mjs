import assert from 'node:assert/strict';
import {linkedActions,diagramNodes,followGame,appliedSave} from '../web/diagrams.mjs';
assert.deepEqual(linkedActions({A:'south',Shake:'south',B:'none'},'south'),['A','Shake']);
assert.deepEqual(linkedActions({A:'none'},'none'),[]);
const actions=['A','B','1','2','Start','Select','Home','Up','Down','Left','Right','Shake','Tilt left','Tilt right','Tilt forward','Tilt backward'];
assert.equal(diagramNodes('wii-sideways',actions).length,actions.length);
assert.equal(new Set(diagramNodes('wii-sideways',actions).map(x=>x.key)).size,actions.length);
assert.equal(followGame('SMNE01','RSBE01',false),'RSBE01');
assert.equal(followGame('SMNE01','RSBE01',true),null);
assert.equal(followGame('SMNE01',null,false),null);
assert.equal(followGame('SMNE01','SMNE01',false),null);
console.log('diagram coverage, shared controls, unknown inputs and safe game following passed');

assert.equal(appliedSave({applied_revision:3,desired_revision:3,apply_phase:'idle',pending:false},4),false);
assert.equal(appliedSave({applied_revision:4,desired_revision:4,apply_phase:'waiting',pending:true},4),false);
assert.equal(appliedSave({applied_revision:4,desired_revision:4,apply_phase:'error',pending:false},4),false);
assert.equal(appliedSave({applied_revision:4,desired_revision:4,apply_phase:'idle',pending:false},4),true);

const {sessionSummary,scopeNote,saveMessage}=await import('../web/diagrams.mjs');
const dsOpen={games:[{id:'ASME',title:'Super Mario 64 DS',system:'ds'}],running:['melonDS'],runtime:{game:null,message:'unknown'},last_game:'ASME'};
assert.match(sessionSummary(dsOpen).title,/melonDS open/);
assert.match(sessionSummary(dsOpen).detail,/loaded DS game is not identified/);
assert.doesNotMatch(sessionSummary(dsOpen).detail,/launch.*Studio/i);
assert.equal(sessionSummary({...dsOpen,runtime:{game:'ASME'}}).title,'Super Mario 64 DS');
assert.equal(sessionSummary({...dsOpen,running:[],runtime:{game:null}}).title,'Last played · Super Mario 64 DS');
assert.match(scopeNote(null,false),/shared/i);
assert.match(scopeNote(dsOpen.games[0],true),/Direct melonDS use keeps its shared controls/);
assert.doesNotMatch(saveMessage('ds',''),/launch/i);
assert.match(saveMessage('ds','ASME'),/optional Studio launch/);
