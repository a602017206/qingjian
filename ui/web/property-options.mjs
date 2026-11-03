export function propertyOptionItems(options,query='',allowCreate=true){
 const text=String(query).trim(),normalized=text.toLocaleLowerCase();
 const items=options.filter(option=>option.label.toLocaleLowerCase().includes(normalized)).map(option=>({...option,kind:'option',key:'option:'+option.id}));
 if(allowCreate&&text&&!options.some(option=>option.label.toLocaleLowerCase()===normalized))items.push({id:text,label:text,kind:'create',key:'create'});
 return items;
}
export function nextOptionKey(items,key,direction){
 if(!items.length)return '';
 const index=items.findIndex(item=>item.key===key),step=direction<0?-1:1;
 return items[index<0?(step>0?0:items.length-1):(index+step+items.length)%items.length].key;
}
