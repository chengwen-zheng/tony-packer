//index.js:
 (globalThis || window || global)['__farm_default_namespace__'] = {__FARM_TARGET_ENV__: 'browser'};(function(r,e){var t={};function n(r){return Promise.resolve(o(r))}function o(e){if(t[e])return t[e].exports;var i={id:e,exports:{}};t[e]=i;r[e](i,i.exports,o,n);return i.exports}o(e)})({"ec853507":function  (_,e,l,n){console.log("runtime/index.js")(globalThis||window||global).__farm_default_namespace__.__farm_module_system__.setPlugins([]);},},"ec853507");(function(_){for(var r in _){_[r].__farm_resource_pot__='index_ddf1.js';(globalThis || window || global)['__farm_default_namespace__'].__farm_module_system__.register(r,_[r])}})({"05ee5ec7":function  (t,e,n,i){"use strict";function r(t){return"number"==typeof t&&!isNaN(t);}function h(t,e,n,i){var h=n,d=i;if(e){var o,u=(o=getComputedStyle(t),{width:(t.clientWidth||parseInt(o.width,10))-parseInt(o.paddingLeft,10)-parseInt(o.paddingRight,10),height:(t.clientHeight||parseInt(o.height,10))-parseInt(o.paddingTop,10)-parseInt(o.paddingBottom,10)});h=u.width?u.width:h,d=u.height?u.height:d;}return{width:Math.max(r(h)?h:1,1),height:Math.max(r(d)?d:1,1)};}function d(t){var e=t.parentNode;e&&e.removeChild(t);}Object.defineProperty(e,"__esModule",{value:!0}),function(t,e){for(var n in e)Object.defineProperty(t,n,{enumerable:!0,get:e[n]});}(e,{getChartSize:function(){return h;},removeDom:function(){return d;}});},"b5d64806":function  (e,o,t,r){"use strict";Object.defineProperty(o,"__esModule",{value:!0});var c=t("05ee5ec7");console.log(c.getChartSize,c.removeDom);},});(globalThis || window || global)['__farm_default_namespace__'].__farm_module_system__.setInitialLoadedResources([]);(globalThis || window || global)['__farm_default_namespace__'].__farm_module_system__.setDynamicModuleResourcesMap({  });var farmModuleSystem = (globalThis || window || global)['__farm_default_namespace__'].__farm_module_system__;farmModuleSystem.bootstrap();var entry = farmModuleSystem.require("b5d64806");