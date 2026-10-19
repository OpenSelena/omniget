const n=new Map;function r(e,t){return n.set(e,t),()=>{n.get(e)===t&&n.delete(e)}}function a(){return Object.fromEntries([...n].map(([e,t])=>[e,t()]))}export{a,r};
