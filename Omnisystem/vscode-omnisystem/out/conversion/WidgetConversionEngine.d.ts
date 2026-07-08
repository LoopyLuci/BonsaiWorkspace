import { ConversionResult, SourceLanguage } from './WidgetIR';
export declare function detectLanguage(source: string, hint?: string): SourceLanguage;
export interface ConversionInput {
    source: string;
    sourceLang: string;
    targetLang: string;
    widgetNameHint?: string;
}
export declare function convert(input: ConversionInput): ConversionResult;
export declare function renderOWPreview(kind: string, name: string): string;
//# sourceMappingURL=WidgetConversionEngine.d.ts.map