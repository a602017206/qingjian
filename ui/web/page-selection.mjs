import { descendants } from './core.mjs';
export function selectedPageIds(pages,selection,deleted=false){const ids=new Set();for(const id of selection){const page=pages.find(p=>p.id===id&&!!p.deleted===deleted);if(!page)continue;for(const childId of descendants(pages,id)){const child=pages.find(p=>p.id===childId);if(child&&!!child.deleted===deleted)ids.add(childId);}}return [...ids];}
