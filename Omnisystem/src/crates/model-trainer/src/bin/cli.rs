//! Model-trainer CLI: builds a small dataset, splits it, trains a model,
//! then validates predictions against actual labels.

use model_trainer::{Dataset, Model, ModelType, Sample, Trainer, Validator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = Dataset::new("demo".to_string());
    for i in 0..10 {
        dataset.add_sample(Sample {
            id: format!("s{}", i),
            features: vec![i as f32],
            label: Some((i % 2) as f32),
        })?;
    }
    let (train, test) = dataset.split(0.8);
    println!(
        "Dataset '{}' split into {} train / {} test samples",
        dataset.name(),
        train.len(),
        test.len()
    );

    let trainer = Trainer::new();
    trainer.create_model(Model::new(
        "demo-model".to_string(),
        "Demo Classifier".to_string(),
        ModelType::LogisticRegression,
    ))?;
    trainer.train("demo-model", 5)?;
    let model = trainer.get_model("demo-model")?;
    println!(
        "Model '{}' trained={} accuracy={:.3} ({} history record(s))",
        model.name,
        model.trained,
        model.accuracy,
        trainer.history_length()
    );

    let predictions = vec![1.0, 0.0, 1.0, 1.0];
    let actual = vec![1.0, 0.0, 0.0, 1.0];
    let accuracy = Validator::calculate_accuracy(&predictions, &actual)?;
    let loss = Validator::calculate_loss(&predictions, &actual)?;
    println!("Validation: accuracy={:.2} loss={:.2}", accuracy, loss);

    Ok(())
}
