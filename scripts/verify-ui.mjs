import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const work = path.join(root, 'release', 'qa');
await mkdir(work, {recursive:true});
const dist = path.join(root, 'dist');
const fixture = await readFile(path.join(root, 'tests', 'fixtures', 'docs-fixture.js'), 'utf8');
const extension = `
mods.forEach((m,i)=>{ m.path='C:/ConanForge/library/transfer-'+i+'/'+m.name+'.pak';m.files[0].path=m.path; });
profile.entries=mods.slice(0,1).map(m=>({path:m.path,enabled:true}));
let acquisitions=mods.map((m,i)=>({digest:'bundle-'+i,path:m.path.slice(0,m.path.lastIndexOf('/')),provider:'steam',modId:'123',fileId:'100',name:m.name,created:1}));
mods.push({...mods[2],name:'ExternalCopy',path:'D:/Game/Mods/ExternalCopy.pak'});
const originalInvoke=window.__TAURI_INTERNALS__.invoke;
window.__TAURI_INTERNALS__.invoke=async(command,args)=>{
 const a=args?.action;
 if(a?.type==='bootstrap'){const result=await originalInvoke(command,args);result.acquisitions=acquisitions;return result;}
 if(a?.type==='online'&&a.action.kind==='check_update'){
  window.__docsCalls.push({command,action:a});return {digest:a.action.digest,state:'available',message:'The Workshop item has changed since this copy was downloaded. Review the source page before downloading an update.',checkedAt:1795000000};
 }
 if(a?.type==='preview_library_delete'){
  window.__docsCalls.push({command,action:a});const found=acquisitions.find(x=>x.digest===a.digest);
  return {token:'fixture-'+a.digest,digest:a.digest,folder:found.path,packages:[found.name],bytes:123,fileCount:1,profiles:a.digest==='bundle-0'?['Desert adventures']:[]};
 }
 if(a?.type==='delete_library'){
  if(!a.confirmed||a.token!=='fixture-bundle-1')throw Error('Unexpected deletion request');
  window.__docsCalls.push({command,action:a});acquisitions=acquisitions.filter(x=>x.digest!=='bundle-1');catalog.mods=catalog.mods.filter(x=>x.name!=='TravelCompass');return 'Library copy deleted (fixture only).';
 }
 return originalInvoke(command,args);
};`;
const server = createServer(async (req,res)=>{
 try {
  const pathname = new URL(req.url, 'http://127.0.0.1').pathname;
  const target = path.resolve(dist, '.'+(pathname==='/'?'/index.html':pathname));
  assert(target.startsWith(dist+path.sep));
  let data=await readFile(target);
  if(target.endsWith('index.html')) data=Buffer.from(data.toString().replace('<head>', '<head><script>'+fixture+extension+'</script>'));
  res.setHeader('Content-Type',target.endsWith('.js')?'text/javascript':target.endsWith('.css')?'text/css':target.endsWith('.png')?'image/png':'text/html');
  res.end(data);
 }catch {res.writeHead(404).end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const profileDir=await mkdtemp(path.join(work,'library-ui-browser-'));
const browser=spawn(process.env.CHROME_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe',[
 '--headless=new','--remote-debugging-port=0','--user-data-dir='+profileDir,'--no-first-run','--no-default-browser-check','about:blank'
],{windowsHide:true,stdio:'ignore'});
let socket;
try {
 let port;
 for(let i=0;i<100;i++){
  try {port=(await readFile(path.join(profileDir,'DevToolsActivePort'),'utf8')).split('\n')[0];break;}catch{await new Promise(r=>setTimeout(r,100));}
 }
 assert(port,'Headless browser did not start');
 const pages=await (await fetch('http://127.0.0.1:'+port+'/json/list')).json();
 socket=new WebSocket(pages.find(p=>p.type==='page').webSocketDebuggerUrl);
 await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
 let nextId=0;const calls=new Map();
 socket.addEventListener('message',event=>{const result=JSON.parse(event.data);if(result.id){const pending=calls.get(result.id);calls.delete(result.id);clearTimeout(pending.timer);result.error?pending.reject(result.error):pending.resolve(result.result);}});
 const call=(method,params={})=>new Promise((resolve,reject)=>{
  const id=++nextId;const timer=setTimeout(()=>{calls.delete(id);reject(Error('Timed out: '+method));},20000);
  calls.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));
 });
 await call('Emulation.setDeviceMetricsOverride',{width:1440,height:1240,deviceScaleFactor:1,mobile:false});
 await call('Page.navigate',{url:'http://127.0.0.1:'+server.address().port});
 await new Promise(r=>setTimeout(r,1200));
 const result=await call('Runtime.evaluate',{awaitPromise:true,returnByValue:true,expression:`(async()=>{
  const assert=(v,m)=>{if(!v)throw Error(m)};
  const wait=async(f)=>{for(let i=0;i<100;i++){if(f())return;await new Promise(r=>setTimeout(r,30));}throw Error('UI did not reach expected state');};
  const button=(text,root=document)=>[...root.querySelectorAll('button')].find(b=>b.textContent.trim().includes(text));
  await wait(()=>button('Mod library'));
  button('Mod library').click();await wait(()=>document.querySelectorAll('.mod-card').length===4);
  const card=name=>[...document.querySelectorAll('.mod-card')].find(c=>c.querySelector('h3').textContent===name);
  const nav=[...document.querySelectorAll('nav button')].map(b=>b.textContent);
  assert(nav[0].includes('Discover')&&nav[1].includes('Mod library')&&nav[2].includes('Load order'),'Sidebar order');
  assert(button('Delete library copy',card('ExternalCopy')).disabled,'External deletion must be disabled');
  button('Check for updates',card('TravelCompass')).click();await wait(()=>card('TravelCompass').textContent.includes('Update available'));
  assert(!window.__docsCalls.some(c=>c.action?.type==='apply'||c.action?.type==='delete_library'),'Checking must not apply or delete');
  button('Delete library copy',card('DesertStructures')).click();await wait(()=>document.querySelector('dialog[open]'));
  assert(document.querySelector('dialog').textContent.includes('Desert adventures'),'Blocking profile is visible');
  assert(button('Delete library copy permanently',document.querySelector('dialog')).disabled,'Used bundle blocked');
  button('Cancel',document.querySelector('dialog')).click();await wait(()=>!document.querySelector('dialog'));
  button('Delete library copy',card('TravelCompass')).click();await wait(()=>document.querySelector('dialog[open]'));
  button('Cancel',document.querySelector('dialog')).click();await wait(()=>!document.querySelector('dialog'));
  assert(!window.__docsCalls.some(c=>c.action?.type==='delete_library'),'Cancel must not delete');
  button('Delete library copy',card('TravelCompass')).click();await wait(()=>document.querySelector('dialog[open]'));
  button('Delete library copy permanently',document.querySelector('dialog')).click();
  await wait(()=>!document.querySelector('dialog')&&!card('TravelCompass'));
  assert(window.__docsCalls.filter(c=>c.action?.type==='delete_library').length===1,'Exactly one confirmed delete');
  assert(!window.__docsCalls.some(c=>c.action?.type==='apply'),'No game apply');
  return {checks:['sidebar order','external controls disabled','update result','no automatic apply','profile blocker','cancel','confirmed delete and refresh'],cards:document.querySelectorAll('.mod-card').length};
 })()`});
 assert(!result.exceptionDetails,JSON.stringify(result.exceptionDetails));
 const prepare=await call('Runtime.evaluate',{awaitPromise:true,returnByValue:true,expression:`(async()=>{
  const buttons=()=>[...document.querySelectorAll('button')];
  buttons().find(b=>b.textContent.includes('Scan')).click();
  await new Promise(r=>setTimeout(r,150));
  const card=[...document.querySelectorAll('.mod-card')].find(c=>c.querySelector('h3').textContent==='CraftingTools');
  [...card.querySelectorAll('button')].find(b=>b.textContent.includes('Check for updates')).click();
  await new Promise(r=>setTimeout(r,150));
  return true;
 })()`});
 assert(!prepare.exceptionDetails,JSON.stringify(prepare.exceptionDetails));
 const screenshot=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
 await writeFile(path.join(work,'07-library-maintenance.png'),Buffer.from(screenshot.data,'base64'));
 const nexus=await call('Runtime.evaluate',{awaitPromise:true,returnByValue:true,expression:`(async()=>{
  const button=t=>[...document.querySelectorAll('button')].find(b=>b.textContent.includes(t));
  button('Discover & import').click();
  await new Promise(r=>setTimeout(r,200));
  button('Nexus Mods').click();
  await new Promise(r=>setTimeout(r,100));
  if(!document.body.textContent.includes('Download on Nexus, then import here'))throw Error('Missing manual download guidance');
  if(document.querySelector('input[type="password"]')||button('Look up')||button('Recently added'))throw Error('Public Nexus API controls must be hidden');
  if(!button('Browse website')||!button('Import ZIP / PAK'))throw Error('Missing website or import control');
  if(window.__docsCalls.some(c=>c.action?.action?.kind==='connect'||c.action?.action?.kind==='lookup'))throw Error('Unexpected provider call');
  return ['public Nexus instructions','no API key or lookup controls','website and manual import available','no provider connection on navigation'];
 })()`});
 assert(!nexus.exceptionDetails,JSON.stringify(nexus.exceptionDetails));
 result.result.value.checks.push(...nexus.result.value);
 const nexusScreenshot=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
 await writeFile(path.join(work,'08-nexus-public.png'),Buffer.from(nexusScreenshot.data,'base64'));
 const appUpdates=await call('Runtime.evaluate',{awaitPromise:true,returnByValue:true,expression:`(async()=>{
  const assert=(v,m)=>{if(!v)throw Error(m)};
  const wait=async(f)=>{for(let i=0;i<100;i++){if(f())return;await new Promise(r=>setTimeout(r,30));}throw Error('Update UI did not settle');};
  const button=(t,root=document)=>[...root.querySelectorAll('button')].find(b=>b.textContent.includes(t));
  const updates=()=>window.__docsCalls.filter(c=>c.command==='app_update');
  button('App updates').click();await wait(()=>document.body.textContent.includes('You’re up to date'));
  assert(updates().filter(c=>c.action.kind==='check').length===1,'One startup check');
  assert(!updates().some(c=>['download','install'].includes(c.action.kind)),'No automatic download/install');
  window.__appUpdateTest.error='Offline test: try again later';
  button('Check for app updates').click();await wait(()=>document.body.textContent.includes('Offline test'));
  window.__appUpdateTest.error=null;
  window.__appUpdateTest.next={version:'0.6.0',notes:'Example future release. Clearer library tools and quality improvements.',downloaded:false};
  button('Check for app updates').click();await wait(()=>button('Download app update'));
  button('Download app update').click();await wait(()=>button('Install and restart…'));
  assert(!updates().some(c=>c.action.kind==='install'),'Download does not install');
  button('Mod library').click();await wait(()=>document.querySelectorAll('.mod-card').length===3);
  const card=[...document.querySelectorAll('.mod-card')].find(c=>c.querySelector('h3').textContent==='CraftingTools');
  button('+ Add to profile',card).click();button('App updates').click();
  await wait(()=>document.body.textContent.includes('Save or discard your profile'));
  assert(button('Install and restart…').disabled,'Unsaved profile blocks install');
  button('Mod library').click();await wait(()=>button('Discard edits'));button('Discard edits').click();button('App updates').click();
  await wait(()=>button('Install and restart…')?.disabled===false);
  button('Install and restart…').click();await wait(()=>document.querySelector('dialog[open]'));
  assert(button('Install and restart',document.querySelector('dialog')).disabled,'Explicit checkbox required');
  button('Not now').click();await wait(()=>!document.querySelector('dialog'));
  assert(!updates().some(c=>c.action.kind==='install'),'Not now does not install');
  button('Install and restart…').click();await wait(()=>document.querySelector('dialog[open]'));
  document.querySelector('dialog input[type=checkbox]').click();
  button('Install and restart',document.querySelector('dialog')).click();await wait(()=>document.body.textContent.includes('finish the active import'));
  assert(updates().filter(c=>c.action.kind==='install').length===1,'One confirmed installation request');
  const preference=document.querySelector('.update-preference input');preference.click();
  await wait(()=>window.__appUpdateTest.checkAtStartup===false);
  assert(!window.__docsCalls.some(c=>c.action?.type==='apply'),'App updates do not apply mods');
  await wait(()=>!preference.disabled && !preference.checked);
  preference.click();await wait(()=>window.__appUpdateTest.checkAtStartup===true && !preference.disabled);
  return ['one startup check','no automatic download/install','offline check recovery','download separate from install','unsaved changes block install','confirmation and Not now','native install refusal is recoverable','startup preference saved'];
 })()`});
 assert(!appUpdates.exceptionDetails,JSON.stringify(appUpdates.exceptionDetails));
 result.result.value.checks.push(...appUpdates.result.value);
 const appScreenshot=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
 await writeFile(path.join(work,'09-app-updates.png'),Buffer.from(appScreenshot.data,'base64'));
 await writeFile(path.join(work,'ui-results.json'),JSON.stringify(result.result.value,null,2));
 console.log(JSON.stringify(result.result.value));
 await call('Browser.close');
} finally {socket?.close();browser.kill();server.close();}
