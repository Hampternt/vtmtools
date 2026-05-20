export function deriveBonusName(meritName: string): string {
  const trimmed = meritName.trim();
  if (trimmed === '') return '';
  return `${trimmed} bonus`;
}

export function isAutoDerived(currentBonusName: string, meritName: string): boolean {
  return currentBonusName === deriveBonusName(meritName);
}
