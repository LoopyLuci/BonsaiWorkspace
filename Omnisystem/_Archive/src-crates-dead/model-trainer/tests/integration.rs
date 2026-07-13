use model_trainer::*;

#[test]
fn test_full_training_workflow() {
    let trainer = trainer::Trainer::new();
    let dataset = dataset::Dataset::new("training_data".to_string());
    
    for i in 0..20 {
        let sample = dataset::Sample {
            id: format!("sample_{}", i),
            features: vec![i as f32, (i * 2) as f32],
            label: Some((i % 2) as f32),
        };
        dataset.add_sample(sample).unwrap();
    }
    
    let (train, test) = dataset.split(0.8);
    assert_eq!(train.len(), 16);
    assert_eq!(test.len(), 4);
    
    let model = Model::new(
        "nn_model".to_string(),
        "Neural Network".to_string(),
        ModelType::NeuralNetwork,
    );
    trainer.create_model(model).unwrap();
    
    trainer.train("nn_model", 5).unwrap();
    
    let trained_model = trainer.get_model("nn_model").unwrap();
    assert!(trained_model.trained);
    assert!(trained_model.accuracy > 0.0);
}
