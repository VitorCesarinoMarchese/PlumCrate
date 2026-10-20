import {test} from 'node:test';
import assert from 'node:assert/strict';
import {currentSectionIndex} from '../public/guide-navigation.js';

test('starts with the first section before its heading reaches the reading line',()=>{
  assert.equal(currentSectionIndex([500,1200,1900],160,false),0);
});
test('keeps a long section current until the next section reaches the reading line',()=>{
  assert.equal(currentSectionIndex([-2200,300,900],160,false),0);
  assert.equal(currentSectionIndex([-2200,160,900],160,false),1);
});
test('scrolling upward returns to the preceding section',()=>{
  assert.equal(currentSectionIndex([-1000,-100,400],160,false),1);
  assert.equal(currentSectionIndex([-500,250,750],160,false),0);
});
test('the final section becomes current at the end even if it is short',()=>{
  assert.equal(currentSectionIndex([-2500,-500,450],160,true),2);
});
