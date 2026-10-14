import { escapeHTML as e } from './core.mjs';
import { icon } from './icons.mjs';
export const pageColors=['default','gray','brown','orange','yellow','green','blue','purple','pink','red'];
export const colorClass=(value,scope='text')=>pageColors.includes(value)&&value!=='default'?`${scope}-${value}`:'';
export function pageLinkTarget(block,pages){
 if(!['paragraph','page'].includes(block.type)||block.pagePresentation==='inline')return null;
 return linkedPage(block.text,pages);
}
export function linkedPage(text,pages){
 const value=String(text||'').trim(),id=value.match(/^\[[^\]\n]*\]\(page:([a-zA-Z0-9_-]+)\)$/)?.[1],title=value.match(/^\[\[([^\]\n]+)\]\]$/)?.[1];
 return pages.find(p=>!p.deleted&&(id?p.id===id:title&&p.title===title))||null;
}
export function pageIcon(page,override=''){
 const value=(page.iconCustomized?page.icon:override||page.icon)||'📄';
 if(/^asset:[a-zA-Z0-9_.-]+$/.test(value))return `<span class="page-icon-asset" data-asset="${e(value.slice(6))}"></span>`;
 return /^https?:\/\//i.test(value)?icon('page',20):`<span class="page-icon-text">${e(value.slice(0,64))}</span>`;
}
export function renderPageLink(page,block={}){
 return `<a class="page-reference ${colorClass(block.textColor||page.textColor)} ${colorClass(block.backgroundColor||page.backgroundColor,'background')}" href="page:${e(page.id)}" data-page-reference="${e(page.id)}">${pageIcon(page,block.linkIcon)}<span class="page-reference-title">${e(page.title)}</span></a>`;
}
