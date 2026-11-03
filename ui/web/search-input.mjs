export function bindSearchInput(input,{update,refresh,delay=0}){
 let timer,composing=false,lastValue=input.value,pending=false;
 const publish=()=>{if(!input.isConnected||composing)return;pending=false;refresh();};
 function apply(){
  if(!input.isConnected||composing)return;
  if(input.value!==lastValue){lastValue=input.value;update(lastValue);pending=true;}
  if(!pending)return;clearTimeout(timer);if(delay)timer=setTimeout(publish,delay);else publish();
 }
 input.oninput=event=>{if(!event.isComposing&&!composing)apply();};
 input.oncompositionstart=()=>{composing=true;clearTimeout(timer);};
 input.oncompositionend=()=>{composing=false;apply();};
 return ()=>clearTimeout(timer);
}
