const e=new WeakMap;function a(t,n){return e.set(t,n),{destroy(){e.delete(t)}}}function o(t){return e.get(t)??[]}export{a,o as b};
