import { t } from './i18n.mjs';
export function pageMenuItems(page,{block=false,ownerLocked=false,canWindow=true}={}){return [
 ...(block?[{action:'pageMenuConvert',label:t('convertTo'),icon:'sync',submenu:true,disabled:ownerLocked}]:[]),
 {action:'pageMenuColor',label:t('color'),icon:'palette',submenu:true,disabled:block?ownerLocked:!!page.locked},
 {action:'pageMenuIcon',label:t('editPageIcon'),icon:'smile',disabled:!!page.locked},
 {action:'pageMenuFavorite',label:t(page.favorite?'unfavorite':'favorite'),icon:'star'},
 {action:'pageMenuRename',label:t('renamePage'),icon:'edit',shortcut:'⇧⌘R',disabled:!!page.locked},
 {separator:true},
 {action:'pageMenuOpen',label:t('openEdit'),icon:'page'},
 {action:'pageMenuTab',label:t('openPageTab'),icon:'arrowUpRight',shortcut:'⇧⌘↵'},
 {action:'pageMenuWindow',label:t('openPageWindow'),icon:'window',disabled:!canWindow,hint:!canWindow?t('desktopWindowOnly'):''},
 {action:'pageMenuPeek',label:t('openPagePeek'),icon:'panel',shortcut:t('altClick')},
 {separator:true},
 {action:'pageMenuCopyLink',label:t('copyPageLink'),icon:'link'},
 {action:'pageMenuDuplicate',label:t('duplicate'),icon:'copy',shortcut:'⌘D'},
 {action:'pageMenuMove',label:t('movePage'),icon:'folder',shortcut:'⇧⌘P',disabled:!!page.locked},
 {action:'pageMenuTrash',label:t('moveTrash'),icon:'trash',danger:true},
 {separator:true},
 {action:'pageMenuMore',label:t('more'),icon:'dots'}
];}
