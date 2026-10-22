export function createDesktopActions({listen,pending,complete,perform,failed}){
 let running=null,again=false,unlisten=null;const handled=new Map();
 function drain(){again=true;if(running)return running;running=(async()=>{do{again=false;let requests;try{requests=await pending();}catch(error){failed(error);return;}for(const request of requests){let success=handled.get(request.id);if(success===undefined){try{await perform(request.action,request.id);success=true;}catch(error){success=false;failed(error);}handled.set(request.id,success);}try{await complete(request.id,success);handled.delete(request.id);}catch(error){failed(error);return;}}}while(again);})().finally(()=>{running=null;});return running;}
 return {async start(){unlisten=await listen(()=>{drain();});await drain();},idle:()=>running||Promise.resolve(),stop:()=>{unlisten?.();}};
}
