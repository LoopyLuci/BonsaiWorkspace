import { LanguageDef, LanguageFamily } from './ULIR';
declare const LANGUAGES: LanguageDef[];
export declare function getLang(id: string): LanguageDef | undefined;
export declare function getLangByExtension(ext: string): LanguageDef | undefined;
export declare function getLangsByFamily(family: LanguageFamily): LanguageDef[];
export declare function allLanguages(): LanguageDef[];
export declare function searchLanguages(query: string): LanguageDef[];
export declare function popularLanguages(limit?: number): LanguageDef[];
export declare function getConversionLabel(sourceId: string, targetId: string): string;
export declare function getFileExtension(langId: string): string;
export { LANGUAGES };
//# sourceMappingURL=LanguageRegistry.d.ts.map