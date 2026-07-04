type BlockStyle = 'brace' | 'indent' | 'functional';
declare function blockStyle(lang: string): BlockStyle;
declare function commentPfx(lang: string): string;
declare function usesSemicolon(lang: string): boolean;
interface LitMap {
    T: string;
    F: string;
    N: string;
    and: string;
    or: string;
    not: string;
}
declare function lits(lang: string): LitMap;
export declare function translateBody(originalSource: string, srcLang: string, tgtLang: string, baseIndent?: string): string;
export { blockStyle, commentPfx, usesSemicolon, lits };
//# sourceMappingURL=BodyTranslator.d.ts.map