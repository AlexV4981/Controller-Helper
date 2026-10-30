import test from 'node:test';
import assert from 'node:assert/strict';
import {portRows,portLabel} from '../web/ports.mjs';
const phys='usb-0000:00:14.0-3/input0';
const key='stable:guid:port:'+Buffer.from(phys).toString('hex');
test('saved USB assignment remains visible disconnected',()=>{
 const rows=portRows([], [{key,player:1}]);
 assert.equal(rows[0].port,phys);assert.equal(rows[0].connected,false);
 assert.equal(rows[2].player,3);assert.equal(rows[2].port,'');
});
test('live roles override stale disconnected roles and distinguish Bluetooth',()=>{
 const rows=portRows([{device:{key:'other',usb_port:'usb-0000:00:14.0-4/input0',bus:3},preference:{player:1}},{device:{key:'bt',bus:5},preference:{player:2}}],[{key,player:1}]);
 assert.equal(portLabel(rows[0].port),'USB 4');assert.equal(rows[1].bluetooth,true);assert.equal(rows[1].port,'');
});
test('hub paths stay distinct and malformed identities are ignored',()=>{
 assert.equal(portLabel('usb-0000:00:14.0-3.2/input0'),'USB 3 · hub 2');
 assert.equal(portRows([], [{key:'stable:guid:port:zz',player:1}])[0].port,'');
 assert.equal(portLabel('arbitrary input'),'USB port unknown');
});
test('physical side labels stay attached to the socket when players change',()=>{
 const rows=portRows([{device:{key,usb_port:phys,bus:3},preference:{player:2}}],[],{[phys]:'Left side'});
 assert.equal(rows[1].label,'Left side');
 assert.equal(rows[0].label,'');
});
