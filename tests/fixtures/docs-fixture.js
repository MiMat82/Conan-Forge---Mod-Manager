window.isTauri = true;
window.__docsCalls = [];
window.__appUpdateTest = {next:null,error:null,checkAtStartup:true,installed:true};
const game = "D:\\Games\\Conan Exiles Enhanced";
const names = ["DesertStructures", "TravelCompass", "CraftingTools"];
const mods = names.map((name,i)=>({name,path:"C:\\Mods\\Library\\"+name+".pak",files:[{path:"C:\\Mods\\Library\\"+name+".pak",size:(i+1)*33554432,modified:1}],workshopId:null,warnings:[]}));
let profile = {id:1,name:"Desert adventures",revision:1,entries:mods.slice(0,2).map(m=>({path:m.path,enabled:true}))};
const settings={gameRoot:game,scanRoots:["C:\\Mods\\Local"]};
const catalog={mods,warnings:[],orphanFiles:[]};
const status={nexusApiEnabled:false,connected:false,premium:false,job:{running:false,stage:"Ready",bytes:0,error:null,imported:null},history:[]};
window.__TAURI_INTERNALS__ = {invoke:async(command,args)=>{
  const action=args?.action;
  window.__docsCalls.push(structuredClone({command,action}));
  if(command==='app_update') {
    const state=window.__appUpdateTest;
    if(action.kind==='status')return {currentVersion:'0.5.0',checkAtStartup:state.checkAtStartup,installed:state.installed};
    if(action.kind==='preference'){state.checkAtStartup=action.enabled;return action.enabled;}
    if(action.kind==='check'){if(state.error)throw Error(state.error);return structuredClone(state.next);}
    if(action.kind==='download'){args.progress.onmessage?.({bytes:1024,total:1024});return {...state.next,downloaded:true};}
    if(action.kind==='install')throw Error('Fixture: finish the active import before updating.');
    if(action.kind==='open_releases')return null;
  }
  if(command!=="request") throw Error("Native dialogs are not available in this illustrated demo.");
  let result;
  switch(action.type){
    case "bootstrap": result={profiles:[profile],settings,catalog,dataDir:"C:\\ConanForge\\Workspace",detection:{steamRoots:["C:\\Steam"],gameRoots:[game],workshopRoots:[],warnings:[]},integrations:[]};break;
    case "save_profile":profile={...action.profile,revision:profile.revision+1};result=profile;break;
    case "scan": result=catalog;break;
    case "preview":result={token:"illustrated-demo-only",target:game+"\\ConanSandbox\\Mods\\modlist.txt",profileName:profile.name,before:"*DesertStructures.pak\r\n",after:profile.entries.filter(e=>e.enabled).map(e=>"*"+e.path.split("\\").pop()+"\r\n").join(""),changed:true,warnings:["World saves are not included in the automatic load-list backup."],enabledCount:profile.entries.filter(e=>e.enabled).length,files:profile.entries.filter(e=>e.enabled).map(e=>({source:e.path,target:game+"\\ConanSandbox\\Mods\\"+e.path.split("\\").pop(),size:33554432,copy:true}))};break;
    case "apply":if(!action.confirmed)throw Error("Confirmation required");result={target:game+"\\ConanSandbox\\Mods\\modlist.txt",backupDir:"C:\\ConanForge\\Workspace\\backups\\apply-example",changed:true,message:"Selected mod files and load order applied successfully."};break;
    case "online":if(action.action.kind==="status")result=status;
      else if(action.action.kind==="lookup")result={provider:"steam",id:"1234567890",title:"Example building mod",summary:"Illustrated example only. Use a real Workshop item whose author supports your installed game version.",updated:0,size:33554432,files:[]};
      else throw Error("Provider downloads are not performed by this illustrated demo.");
      break;
    default:throw Error("Not implemented in documentation fixture: "+action.type);
  }
  return structuredClone(result);
}};
window.__TAURI_INTERNALS__.transformCallback=()=>Math.floor(Math.random()*1000000);
window.__TAURI_INTERNALS__.unregisterCallback=()=>{};
