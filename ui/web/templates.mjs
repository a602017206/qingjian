import { uid,makePage,makeDatabase,parseMarkdown } from './core.mjs';
import { t } from './i18n.mjs';
export function templates(){return [
 {key:'databaseDemo',icon:'🧮',description:t('databaseDemoDescription'),make:()=>({...makePage(t('databaseDemo')),icon:'🧮',blocks:[],database:{columns:[{id:'name',name:'事项',type:'text'},{id:'price',name:'单价',type:'number'},{id:'quantity',name:'数量',type:'number'},{id:'total',name:'金额',type:'formula',formula:'prop("单价") * prop("数量")'},{id:'tags',name:'分类',type:'multiselect',options:['计划','学习','工作']},{id:'date',name:'日期',type:'date'}],rows:[{id:uid(),values:{name:'示例记录',price:'25',quantity:'4',tags:'["学习"]',date:new Date().toLocaleDateString('sv-SE')}}]}})},
 {key:'blank',icon:'📄',description:t('blankDescription'),make:()=>makePage(t('emptyTitle'))},
 {key:'journal',icon:'☀️',description:t('journalDescription'),make:()=>({...makePage(new Date().toLocaleDateString('zh-CN')+' · 每日记录'),blocks:parseMarkdown('## 今天的重点\n\n- [ ] 最重要的一件事\n\n## 今日收获\n\n写下一个值得记住的瞬间。\n\n## 明天的计划\n\n- [ ] 下一步行动')})},
 {key:'meeting',icon:'📝',description:t('meetingDescription'),make:()=>({...makePage(t('meeting')),blocks:parseMarkdown('## 会议信息\n\n时间：\n参与人：\n\n## 讨论要点\n\n- 要点一\n\n## 决策与行动\n\n- [ ] 待办事项、负责人和截止日期')})},
 {key:'reading',icon:'📚',description:t('readingDescription'),make:()=>({...makePage(t('reading')),blocks:parseMarkdown('## 书籍信息\n\n书名：\n作者：\n\n## 摘录\n\n> 值得回味的一句话。\n\n## 我的理解\n\n这段内容与已有知识有什么联系？\n\n## 可以实践的事\n\n- [ ] 一个小行动')})},
 {key:'project',icon:'🗂️',description:t('projectDescription'),make:()=>({...makePage(t('project')),icon:'🗂️',database:makeDatabase(),blocks:[]})}
 ];}
