pub mod model;
pub mod evaluator;
pub mod train;
pub mod training_loop;

pub use model::GoNet;
pub use evaluator::NeuralEvaluator;
pub use training_loop::GoTrainingLoop;
