import { ULIRModule, OmniCCConversionRequest, OmniCCConversionResult, ConversionOptions } from './ULIR';
export declare class OmniCCConversionEngine {
    private opts;
    constructor(opts?: Partial<ConversionOptions>);
    convert(req: OmniCCConversionRequest): OmniCCConversionResult;
    private convertSnippet;
    private convertProject;
    parse(source: string, langId: string, opts?: ConversionOptions): ULIRModule;
    generate(ir: ULIRModule, targetLangId: string, opts?: ConversionOptions): string;
    getSupportedLanguages(): Array<{
        id: string;
        name: string;
        family: string;
        canParse: boolean;
        canGenerate: boolean;
    }>;
    getConversionPaths(sourceLangId: string): string[];
}
export declare function createEngine(opts?: Partial<ConversionOptions>): OmniCCConversionEngine;
export declare function quickConvert(source: string, targetLang: string, sourceLang?: string, filename?: string): OmniCCConversionResult;
//# sourceMappingURL=ConversionEngine.d.ts.map