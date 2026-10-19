export const isPhone=()=>window.matchMedia('(max-width:760px)').matches;
export function viewportGeometry(height,viewport){const top=Math.max(0,viewport?.offsetTop||0),visible=Math.max(1,Math.min(height-top,viewport?.height||height));return {top,height:visible,bottom:top+visible,inset:Math.max(0,height-top-visible)};}
export function installMobileViewport(){
 const update=()=>{const frame=viewportGeometry(innerHeight,window.visualViewport),style=document.documentElement.style;style.setProperty('--visible-height',frame.height+'px');style.setProperty('--visible-top',frame.top+'px');style.setProperty('--keyboard-inset',frame.inset+'px');};
 window.addEventListener('resize',update);window.visualViewport?.addEventListener('resize',update);window.visualViewport?.addEventListener('scroll',update);update();
}
