import{a as d,b as v,c as x}from"./chunk-WWI6VYDO.js";var b=d(h=>{"use strict";var O=x(),k=Symbol.for("react.element"),E=Symbol.for("react.fragment"),j=Object.prototype.hasOwnProperty,w=O.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED.ReactCurrentOwner,C={key:!0,ref:!0,__self:!0,__source:!0};function y(s,e,i){var t,r={},l=null,n=null;i!==void 0&&(l=""+i),e.key!==void 0&&(l=""+e.key),e.ref!==void 0&&(n=e.ref);for(t in e)j.call(e,t)&&!C.hasOwnProperty(t)&&(r[t]=e[t]);if(s&&s.defaultProps)for(t in e=s.defaultProps,e)r[t]===void 0&&(r[t]=e[t]);return{$$typeof:k,type:s,key:l,ref:n,props:r,_owner:w.current}}h.Fragment=E;h.jsx=y;h.jsxs=y});var m=d((q,N)=>{"use strict";N.exports=b()});var a=v(m(),1);function R({data:s,title:e,type:i="bar",color:t}){let r=typeof s=="string"?s.split(",").map(o=>parseFloat(o.trim())).filter(o=>!isNaN(o)):s;if(!r||r.length===0)return(0,a.jsx)("div",{className:"chart chart--empty",children:"No data available"});let l=Math.max(...r),n=Math.min(...r),f=l-n||1,_=100/r.length,u=t||"var(--color-primary, #3b82f6)";return(0,a.jsxs)("div",{className:"chart",children:[e&&(0,a.jsx)("h3",{className:"chart__title",children:e}),(0,a.jsx)("svg",{viewBox:"0 0 100 50",className:"chart__svg",preserveAspectRatio:"none",children:i==="bar"?r.map((o,c)=>{let p=(o-n)/f*45+5;return(0,a.jsx)("rect",{x:c*_+_*.1,y:50-p,width:_*.8,height:p,fill:u,rx:"1"},c)}):(0,a.jsx)("polyline",{fill:"none",stroke:u,strokeWidth:"1",points:r.map((o,c)=>{let p=c/(r.length-1)*100,g=50-(o-n)/f*45-2.5;return`${p},${g}`}).join(" ")})}),(0,a.jsxs)("div",{className:"chart__labels",children:[(0,a.jsx)("span",{className:"chart__label chart__label--min",children:n}),(0,a.jsx)("span",{className:"chart__label chart__label--max",children:l})]})]})}export{R as default};
/*! Bundled license information:

react/cjs/react-jsx-runtime.production.min.js:
  (**
   * @license React
   * react-jsx-runtime.production.min.js
   *
   * Copyright (c) Facebook, Inc. and its affiliates.
   *
   * This source code is licensed under the MIT license found in the
   * LICENSE file in the root directory of this source tree.
   *)
*/
