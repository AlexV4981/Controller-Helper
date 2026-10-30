import test from 'node:test';
import assert from 'node:assert/strict';
import {nextPress} from '../web/input.mjs';
test('learning captures a newly pressed face button',()=>assert.equal(nextPress({south:1,east:0},{south:0,east:0}),'south'));
test('a button already held does not hijack learning',()=>assert.equal(nextPress({south:1},{south:1}),null));
test('idle stick drift does not count as an assignment',()=>assert.equal(nextPress({'lx+':.12},{'lx+':.10}),null));
test('learning can intentionally map an analog direction',()=>assert.equal(nextPress({'ly-':.9},{'ly-':.1}),'ly-'));
test('a new button is selected instead of an older held button',()=>assert.equal(nextPress({south:1,east:1},{south:1,east:0}),'east'));
