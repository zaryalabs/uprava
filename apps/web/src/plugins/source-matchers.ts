export function colorLiteralKind(source: string) {
  if (/^#[\da-f]{3,8}$/i.test(source)) return "hex";
  if (
    /^rgba?\(\s*\d{1,3}(?:\s*,\s*\d{1,3}){2}(?:\s*,\s*(?:0|1|0?\.\d+))?\s*\)$/i.test(
      source,
    )
  ) {
    return "rgb";
  }
  if (
    /^hsla?\(\s*-?\d+(?:\.\d+)?(?:deg)?\s*,\s*\d+(?:\.\d+)?%\s*,\s*\d+(?:\.\d+)?%(?:\s*,\s*(?:0|1|0?\.\d+))?\s*\)$/i.test(
      source,
    )
  ) {
    return "hsl";
  }
  return null;
}
