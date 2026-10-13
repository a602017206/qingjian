import { t } from './i18n.mjs';
export function pageMenuItems(page){return [
 {action:'pageMenuOpen',label:t('openEdit'),icon:'page'},
 {action:'pageMenuRename',label:t('renamePage'),icon:'edit',disabled:!!page.locked},
 {action:'pageMenuFavorite',label:t(page.favorite?'unfavorite':'favorite'),icon:'star'},
 {action:'pageMenuDuplicate',label:t('duplicate'),icon:'copy'},
 {action:'pageMenuMove',label:t('movePage'),icon:'folder',disabled:!!page.locked},
 {action:'pageMenuTrash',label:t('moveTrash'),icon:'trash',danger:true}
];}
