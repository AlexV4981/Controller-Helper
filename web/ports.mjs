export function portLabel(port){
 const match=/^usb-[\da-fA-F:.]+-(\d+(?:\.\d+)*)\/input\d+$/.exec(port||'');
 if(!match)return 'USB port unknown';
 const [socket,...hub]=match[1].split('.');
 return `USB ${socket}`+(hub.length?` · hub ${hub.join('.')}`:'');
}
function savedPort(key){
 const hex=key?.split(':port:')[1];
 if(!hex||hex.length>1024||hex.length%2||!/^[\da-f]+$/i.test(hex))return '';
 const value=new TextDecoder().decode(Uint8Array.from(hex.match(/../g),b=>parseInt(b,16)));
 return portLabel(value)==='USB port unknown'?'':value;
}
export function portRows(controllers=[],saved=[],labels={}){
 const rows=Array.from({length:Math.max(3,...controllers.map(c=>c.preference?.player||1),...saved.map(s=>s.player||1))},(_,i)=>({player:i+1,port:'',connected:false,bluetooth:false,key:''})).slice(0,4);
 for(const s of saved){const row=rows[s.player-1],port=savedPort(s.key);if(row&&port&&!row.port)Object.assign(row,{port,key:s.key});}
 for(const c of controllers){const row=rows[(c.preference?.player||1)-1];if(row)Object.assign(row,{port:c.device.usb_port||savedPort(c.device.key),key:c.device.key,connected:true,bluetooth:c.device.bus===5});}
 for(const row of rows)row.label=labels[row.port]||'';
 return rows;
}
