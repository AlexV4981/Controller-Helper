import * as THREE from '/three.module.js';

/** A lightweight, asset-free 3D instrument. Every mesh is also available in
 * the accessible assignment list; the model never owns mapping state. */
export class ControllerView {
  constructor(canvas, onPick) {
    this.canvas = canvas; this.onPick = onPick; this.family = 'xbox'; this.parts = new Map();
    this.sticks = []; this.pickables = []; this.values = {}; this.selected = ''; this.alive = true;
    try { this.renderer = new THREE.WebGLRenderer({canvas, antialias:true, alpha:true, powerPreference:'low-power'}); }
    catch { document.querySelector('#webgl-note').hidden = false; return; }
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
    this.renderer.setClearColor(0x000000,0); this.renderer.outputColorSpace = THREE.SRGBColorSpace; this.renderer.toneMapping=THREE.ACESFilmicToneMapping; this.renderer.toneMappingExposure=1.05;
    this.scene = new THREE.Scene(); this.camera = new THREE.PerspectiveCamera(39,1,.1,100);this.camera.position.z=this.fitDistance||6.7;
    this.scene.add(new THREE.HemisphereLight(0xd4f5e6,0x152228,1.25));
    const key=new THREE.DirectionalLight(0xffedd4,2);key.position.set(-3,5,8);this.scene.add(key);
    const rim=new THREE.DirectionalLight(0x85d6c4,1);rim.position.set(5,-2,2);this.scene.add(rim);
    this.group=new THREE.Group();this.scene.add(this.group);this.ray=new THREE.Raycaster();this.pointer=new THREE.Vector2();
    this.build('xbox');this.reset();
    this.resizeObserver=new ResizeObserver(()=>this.resize());this.resizeObserver.observe(canvas);this.resize();
    let down=null;
    canvas.addEventListener('pointerdown',e=>{if(e.button!==0)return;down={x:e.clientX,y:e.clientY,lastX:e.clientX,lastY:e.clientY,moved:false};canvas.setPointerCapture(e.pointerId);});
    canvas.addEventListener('pointermove',e=>{if(!down)return;const dx=e.clientX-down.lastX,dy=e.clientY-down.lastY;if(Math.hypot(e.clientX-down.x,e.clientY-down.y)>5)down.moved=true;if(down.moved){this.group.rotation.y+=dx*.008;this.group.rotation.x=Math.max(-1.3,Math.min(1.3,this.group.rotation.x+dy*.008));}down.lastX=e.clientX;down.lastY=e.clientY;});
    canvas.addEventListener('pointerup',e=>{if(down&&!down.moved){const control=this.pick(e.clientX,e.clientY);if(control)this.onPick(control);}down=null;});
    canvas.addEventListener('pointercancel',()=>down=null);
    canvas.addEventListener('wheel',e=>{e.preventDefault();this.camera.position.z=Math.max(5.7,Math.min(13,this.camera.position.z+e.deltaY*.005));},{passive:false});
    canvas.addEventListener('keydown',e=>{const key=e.key;if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','+','-','=','Home'].includes(key))e.preventDefault();if(key==='ArrowLeft')this.group.rotation.y-=.12;if(key==='ArrowRight')this.group.rotation.y+=.12;if(key==='ArrowUp')this.group.rotation.x-=.12;if(key==='ArrowDown')this.group.rotation.x+=.12;if(key==='+'||key==='=')this.camera.position.z=Math.max(5.7,this.camera.position.z-.3);if(key==='-')this.camera.position.z=Math.min(13,this.camera.position.z+.3);if(key==='Home')this.reset();});
    canvas.addEventListener('dragover',e=>{if(e.dataTransfer.types.includes('text/plain'))e.preventDefault();});
    canvas.addEventListener('drop',e=>{e.preventDefault();const control=this.pick(e.clientX,e.clientY);const action=e.dataTransfer.getData('text/plain');if(control&&action)this.onPick(control,action);});
    this.lastFrame=0;this.frame=t=>{if(!this.alive)return;requestAnimationFrame(this.frame);if(document.hidden||t-this.lastFrame<32)return;this.lastFrame=t;this.animate();this.renderer.render(this.scene,this.camera);};requestAnimationFrame(this.frame);
  }
  material(color,metalness=.15,roughness=.45){return new THREE.MeshStandardMaterial({color,metalness,roughness});}
  label(text,color='#f1ede4',size=.34){
    const c=document.createElement('canvas');c.width=c.height=256;const x=c.getContext('2d');x.clearRect(0,0,256,256);x.fillStyle=color;x.textAlign='center';x.textBaseline='middle';x.font=`600 ${text.length>2?55:110}px "DejaVu Sans",sans-serif`;x.fillText(text,128,139);const texture=new THREE.CanvasTexture(c);texture.colorSpace=THREE.SRGBColorSpace;
    return new THREE.Mesh(new THREE.PlaneGeometry(size,size),new THREE.MeshBasicMaterial({map:texture,transparent:true,depthWrite:false}));
  }
  roundedShape(points){const s=new THREE.Shape();s.moveTo(...points[0]);for(let i=1;i<points.length;i+=3)s.bezierCurveTo(...points[i],...points[i+1],...points[i+2]);return s;}
  build(family){
    if(!this.renderer)return;this.family=family;while(this.group.children.length){const child=this.group.children[0];this.group.remove(child);child.traverse(o=>{o.geometry?.dispose();if(o.material){for(const m of Array.isArray(o.material)?o.material:[o.material]){m.map?.dispose();m.dispose();}}});}
    this.parts.clear();this.sticks=[];this.pickables=[];
    const sony=family==='playstation',nintendo=family==='nintendo';
    const points=[[-2.35,.8],[-2.1,1.35],[-1.1,1.18],[-.75,1.1],[-.2,1.04],[.2,1.04],[.75,1.1],[1.1,1.18],[2.1,1.35],[2.35,.8],[2.62,.28],[2.72,-1.3],[2.25,-1.63],[1.9,-1.8],[1.47,-.72],[1.12,-.68],[.65,-.77],[-.65,-.77],[-1.12,-.68],[-1.47,-.72],[-1.9,-1.8],[-2.25,-1.63],[-2.72,-1.3],[-2.62,.28],[-2.35,.8]];
    const shape=this.roundedShape(points);const geom=new THREE.ExtrudeGeometry(shape,{depth:.34,bevelEnabled:true,bevelSegments:4,steps:1,bevelSize:.16,bevelThickness:.14,curveSegments:28});geom.computeVertexNormals();
    const body=new THREE.Mesh(geom,this.material(sony?0xced6d3:nintendo?0x303b3f:0x29343b,.17,.4));body.position.z=-.2;this.group.add(body);
    const inset=new THREE.Mesh(new THREE.ShapeGeometry(shape,28),this.material(sony?0x242f35:0x35434a,.12,.55));inset.scale.set(.91,.90,1);inset.position.set(0,.02,.31);this.group.add(inset);
    // Add the seams and center light.
    const curve=new THREE.EdgesGeometry(geom,40);const seam=new THREE.LineSegments(curve,new THREE.LineBasicMaterial({color:sony?0x7c9492:0x78988d,transparent:true,opacity:.2}));seam.position.z=-.195;this.group.add(seam);
    const strip=new THREE.Mesh(new THREE.BoxGeometry(.55,.025,.025),this.material(0x9dd9bf,.1,.35));strip.position.set(0,.53,.36);this.group.add(strip);
    if(sony){const touch=new THREE.Mesh(new THREE.BoxGeometry(1.23,.61,.07),this.material(0x2e3b43));touch.position.set(0,.72,.36);this.group.add(touch);}
    const labelMap=sony?{south:'✕',east:'○',west:'□',north:'△',lb:'L1',rb:'R1',lt:'L2',rt:'R2',back:'⋮',start:'≡',guide:'PS'}:nintendo?{south:'B',east:'A',west:'Y',north:'X',lb:'L',rb:'R',lt:'ZL',rt:'ZR',back:'−',start:'+',guide:'⌂'}:{south:'A',east:'B',west:'X',north:'Y',lb:'LB',rb:'RB',lt:'LT',rt:'RT',back:'⧉',start:'≡',guide:'⌂'};
    const colors=sony?{south:'#8ec8f1',east:'#f3a4a7',west:'#dda9d1',north:'#9cdbc5'}:nintendo?{south:'#e3e9e5',east:'#e3e9e5',west:'#e3e9e5',north:'#e3e9e5'}:{south:'#9bddab',east:'#ec9a91',west:'#85c2ef',north:'#e6cd85'};
    const fx=1.5,fy=.48;
    for(const [control,x,y] of [['south',fx,fy-.34],['east',fx+.35,fy],['west',fx-.35,fy],['north',fx,fy+.34]])this.button(control,x,y,.205,labelMap[control],colors[control]);
    const leftX=sony?-.89:-1.48,leftY=sony?-.55:.52,rightX=.84,rightY=-.55;
    this.stick('ls',leftX,leftY,0);this.stick('rs',rightX,rightY,2);
    const dx=sony?-1.56:-.85,dy=sony?.48:-.52;
    for(const [control,x,y,mark] of [['up',dx,dy+.24,'↑'],['down',dx,dy-.24,'↓'],['left',dx-.24,dy,'←'],['right',dx+.24,dy,'→']])this.button(control,x,y,.16,mark,'#a9c3b9');
    this.button('guide',0,.13,.19,labelMap.guide,'#b8dbce');this.button('back',-.46,.18,.10,labelMap.back,'#bbc7c6');this.button('start',.46,.18,.10,labelMap.start,'#bbc7c6');
    for(const [control,x,y,z,w]of [['lb',-1.55,1.18,.30,.86],['rb',1.55,1.18,.30,.86],['lt',-1.55,1.46,.03,.62],['rt',1.55,1.46,.03,.62]]){
      const part=new THREE.Group();part.position.set(x,y,z);const mesh=new THREE.Mesh(new THREE.BoxGeometry(w,.28,.16),this.material(0x172326));mesh.userData.control=control;part.add(mesh);this.pickables.push(mesh);const label=this.label(labelMap[control],'#b4c8bf',.27);label.position.z=.105;label.userData.control=control;part.add(label);this.pickables.push(label);this.group.add(part);this.parts.set(control,{group:part,mesh,base:z});
    }
    const etch=this.label('PORT','#849f94',.47);etch.position.set(0,-.42,.34);this.group.add(etch);
    this.setSelection(this.selected);
  }
  button(control,x,y,radius,text,color){
    const group=new THREE.Group();group.position.set(x,y,.47);const mesh=new THREE.Mesh(new THREE.CylinderGeometry(radius,radius*.95,.13,36),this.material(0x152126,.2,.3));mesh.rotation.x=Math.PI/2;mesh.userData.control=control;group.add(mesh);this.pickables.push(mesh);
    const ring=new THREE.Mesh(new THREE.TorusGeometry(radius+.012,.016,8,40),this.material(0x53665f,.4,.3));ring.position.z=.005;group.add(ring);
    const label=this.label(text,color,radius*1.5);label.position.z=.078;label.userData.control=control;group.add(label);this.pickables.push(label);this.group.add(group);this.parts.set(control,{group,mesh,base:.47});
  }
  stick(control,x,y,axis){
    const base=new THREE.Mesh(new THREE.TorusGeometry(.34,.085,10,40),this.material(0x101b20,.12,.55));base.position.set(x,y,.42);this.group.add(base);
    const group=new THREE.Group();group.position.set(x,y,.54);const mesh=new THREE.Mesh(new THREE.CylinderGeometry(.285,.24,.13,36),this.material(0x192a2e,.16,.8));mesh.rotation.x=Math.PI/2;mesh.userData.control=control;group.add(mesh);this.pickables.push(mesh);
    const top=new THREE.Mesh(new THREE.TorusGeometry(.22,.012,6,32),this.material(0x60766c));top.position.z=.075;group.add(top);this.group.add(group);this.sticks.push({group,x,y,axis});this.parts.set(control,{group,mesh,base:.54});
  }
  resize(){if(!this.renderer)return;const r=this.canvas.getBoundingClientRect();if(r.width<1||r.height<1)return;this.renderer.setSize(r.width,r.height,false);this.camera.aspect=r.width/r.height;this.fitDistance=Math.max(6.7,3.05/(Math.tan(THREE.MathUtils.degToRad(this.camera.fov/2))*this.camera.aspect));this.camera.position.z=this.fitDistance;this.camera.updateProjectionMatrix();}
  pick(x,y){if(!this.renderer)return null;const r=this.canvas.getBoundingClientRect();this.pointer.set((x-r.left)/r.width*2-1,-(y-r.top)/r.height*2+1);this.ray.setFromCamera(this.pointer,this.camera);return this.ray.intersectObjects(this.pickables,false)[0]?.object.userData.control||null;}
  setSelection(control){this.selected=control;}
  update(device){this.values=device?.values||{};this.axes=device?.axes||[0,0,0,0,0,0];if(device?.family&&device.family!==this.family)this.build(device.family);}
  animate(){for(const [control,p]of this.parts){const amount=this.values[control]||0;p.group.position.z=p.base-amount*.065;p.mesh.material.emissive.setHex(control===this.selected?0x84602b:amount>.4?0x37684f:0);p.mesh.material.emissiveIntensity=control===this.selected?.65:.6;}
    for(const s of this.sticks){s.group.position.x=s.x+(this.axes?.[s.axis]||0)*.1;s.group.position.y=s.y-(this.axes?.[s.axis+1]||0)*.1;}}
  reset(){if(!this.renderer)return;this.group.rotation.set(-.30,-.10,-.025);this.camera.position.z=this.fitDistance||6.7;}
  front(){if(this.renderer)this.group.rotation.set(0,0,0);}
}
