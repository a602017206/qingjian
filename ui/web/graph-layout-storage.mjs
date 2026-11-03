const layoutKey=(workspace,center)=>'qingjian:graph-layout:'+JSON.stringify([workspace,center]);
const validPoint=point=>point&&Number.isFinite(point.x)&&Number.isFinite(point.y)&&point.x>=0&&point.y>=0&&point.x<=100000&&point.y<=100000;

export function loadGraphLayout(storage,workspace,center){
 const raw=storage.getItem(layoutKey(workspace,center)),positions=Object.create(null);
 if(raw===null)return positions;
 const data=JSON.parse(raw);
 if(!data||data.version!==1||!data.positions||typeof data.positions!=='object'||Array.isArray(data.positions))throw new Error('invalid graph layout');
 for(const [id,point] of Object.entries(data.positions)){if(!validPoint(point))throw new Error('invalid graph position');positions[id]={x:point.x,y:point.y};}
 return positions;
}
export function saveGraphPosition(storage,workspace,center,id,point){
 if(!workspace||typeof id!=='string'||!id||!validPoint(point))throw new Error('invalid graph position');
 const positions=loadGraphLayout(storage,workspace,center);positions[id]={x:point.x,y:point.y};
 storage.setItem(layoutKey(workspace,center),JSON.stringify({version:1,positions}));
 return positions;
}
