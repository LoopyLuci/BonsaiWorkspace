import { ULIRModule, ULIRUnit, ConversionOptions } from './ULIR';
import { ConversionResult } from '../conversion/WidgetIR';
export interface WidgetBridgeResult {
    detected: boolean;
    widgetCount: number;
    widgetResults: ConversionResult[];
    mergedNotes: string[];
    uiUnits: ULIRUnit[];
}
export declare function detectUIPatterns(ir: ULIRModule): {
    hasUI: boolean;
    uiUnits: ULIRUnit[];
    confidence: number;
};
export declare function runWidgetBridge(ir: ULIRModule, targetLangId: string, opts: ConversionOptions): WidgetBridgeResult;
export declare function mergeWidgetBridgeResults(mainOutput: string, bridgeResult: WidgetBridgeResult, targetLangId: string): string;
export interface WidgetBridgeSummary {
    detected: boolean;
    widgetCount: number;
    convertedCount: number;
    confidence: number;
    previewHtml: string;
}
export declare function buildBridgeSummary(result: WidgetBridgeResult, ir: ULIRModule): WidgetBridgeSummary;
//# sourceMappingURL=WidgetBridge.d.ts.map