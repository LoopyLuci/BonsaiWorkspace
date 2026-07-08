import { LanguageFamily } from './ULIR';
export interface DetectionResult {
    langId: string;
    name: string;
    family: LanguageFamily;
    confidence: number;
    signals: string[];
}
export declare function detectLanguage(source: string, filenameHint?: string, langHint?: string): DetectionResult;
export declare function detectLanguageBatch(files: Array<{
    path: string;
    content: string;
}>): Map<string, DetectionResult>;
//# sourceMappingURL=LanguageDetector.d.ts.map