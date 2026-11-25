import table from '../data/icons.json';
import type { CatalogItem } from './types';

const ENKA = 'https://enka.network';
const PAIMON = 'https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/static/images';
const key = /^[a-z0-9_]+$/;
const t = table as { element_by_name: Record<string, string>; genshin_icons: Record<string, string>; zenless_icons: Record<string, string> };

const paimonKey = (i: CatalogItem) => (i.paimon_key && key.test(i.paimon_key) ? i.paimon_key : null);
export const elementOf = (name: string) => t.element_by_name[name.trim().toLowerCase()] || 'neutral';

/** Picture addresses to try, best first (small portraits). */
export function iconUrls(item: CatalogItem | undefined | null): string[] {
  if (!item || !/^\d+$/.test(item.id)) return [];
  const weapon = item.kind !== 'character';
  if (item.game === 'hkrpg') return [`${ENKA}/ui/hsr/SpriteOutput/${weapon ? 'LightConeMaxFigures' : 'AvatarRoundIcon/Avatar'}/${item.id}.png`];
  if (item.game === 'hk4e') {
    const k = t.genshin_icons[item.id];
    const p = paimonKey(item);
    return [
      ...(k ? (weapon ? [`${ENKA}/ui/UI_EquipIcon_${k}.png`] : [`${ENKA}/ui/UI_AvatarIcon_${k}.png`, `${ENKA}/ui/UI_AvatarIcon_Side_${k}.png`]) : []),
      ...(p ? [`${PAIMON}/${weapon ? 'weapons' : 'characters'}/${p}.png`] : [])
    ];
  }
  if (item.game === 'nap') {
    const k = t.zenless_icons[item.id];
    return k ? [`${ENKA}/ui/zzz/${k}.png`] : [];
  }
  return [];
}

/** Picture addresses to try for a banner's background, best first (large artwork before small portraits). */
export function artUrls(item: CatalogItem | undefined | null): string[] {
  if (!item || !/^\d+$/.test(item.id)) return [];
  const weapon = item.kind !== 'character';
  if (item.game === 'hk4e') {
    const k = t.genshin_icons[item.id];
    const p = paimonKey(item);
    if (weapon) return [...(k ? [`${ENKA}/ui/UI_Gacha_EquipIcon_${k}.png`, `${ENKA}/ui/UI_EquipIcon_${k}.png`] : []), ...(p ? [`${PAIMON}/weapons/${p}.png`] : [])];
    return [
      ...(k ? [`${ENKA}/ui/UI_Gacha_AvatarImg_${k}.png`] : []), ...(p ? [`${PAIMON}/characters/full/${p}.png`] : []),
      ...(k ? [`${ENKA}/ui/UI_AvatarIcon_${k}.png`] : []), ...(p ? [`${PAIMON}/characters/${p}.png`] : [])
    ];
  }
  if (item.game === 'hkrpg') {
    return weapon
      ? [`${ENKA}/ui/hsr/SpriteOutput/LightConeMaxFigures/${item.id}.png`]
      : [`${ENKA}/ui/hsr/SpriteOutput/AvatarDrawCard/${item.id}.png`, `${ENKA}/ui/hsr/SpriteOutput/AvatarRoundIcon/Avatar/${item.id}.png`];
  }
  if (item.game === 'nap') {
    const k = t.zenless_icons[item.id];
    if (!k) return [];
    const portrait = weapon ? null : k.replace('IconRoleCircle', 'IconRole');
    return [...(portrait && portrait !== k ? [`${ENKA}/ui/zzz/${portrait}.png`] : []), `${ENKA}/ui/zzz/${k}.png`];
  }
  return [];
}
