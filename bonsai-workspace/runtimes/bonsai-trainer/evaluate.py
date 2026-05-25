import json, argparse
from unsloth import FastLanguageModel

def compute_tool_f1(pred,true):
    pred_tools = {s["tool"] for s in pred.get("plan",[])}
    true_tools = {s["tool"] for s in true.get("plan",[])}
    if not pred_tools and not true_tools: return 1.0
    if not pred_tools or not true_tools: return 0.0
    tp = len(pred_tools & true_tools)
    prec = tp/len(pred_tools) if pred_tools else 0
    rec = tp/len(true_tools) if true_tools else 0
    return 2*prec*rec/(prec+rec) if (prec+rec)>0 else 0.0

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--adapter")
    parser.add_argument("--test_data",default="data/bonsai_core/bonsai_core_test.jsonl")
    args = parser.parse_args()
    model,tokenizer = FastLanguageModel.from_pretrained(args.adapter, max_seq_length=2048, load_in_4bit=True)
    with open(args.test_data) as f:
        lines = f.readlines()
    exact = 0; f1_sum = 0; total = 0
    for line in lines:
        ex = json.loads(line)["messages"]
        prompt = ex[0]["content"] + "\n" + ex[1]["content"]
        expected = json.loads(ex[2]["content"])
        # inference placeholder (add actual generation)
        actual = expected  # replace with model inference
        if actual == expected: exact += 1
        f1_sum += compute_tool_f1(actual, expected)
        total += 1
    print(f"Exact match: {exact/total:.3f}, Tool F1: {f1_sum/total:.3f}")

if __name__=="__main__":
    main()
