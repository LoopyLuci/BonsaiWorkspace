import 'package:flutter/material.dart';
import '../services/api_service.dart';
import '../models/skill.dart';

class ExerciseScreen extends StatefulWidget {

  const ExerciseScreen({Key? key, required this.exerciseId}) : super(key: key);
  final String exerciseId;

  @override
  State<ExerciseScreen> createState() => _ExerciseScreenState();
}

class _ExerciseScreenState extends State<ExerciseScreen> {
  late Future<Exercise> _exerciseFuture;
  final _answerController = TextEditingController();
  final _apiService = ApiService();
  String? _feedback;
  bool _isSubmitting = false;

  @override
  void initState() {
    super.initState();
    _exerciseFuture = _apiService.fetchExercise(widget.exerciseId);
  }

  @override
  void dispose() {
    _answerController.dispose();
    super.dispose();
  }

  Future<void> _submitAnswer() async {
    setState(() => _isSubmitting = true);

    try {
      final result = await _apiService.submitAttempt(
        widget.exerciseId,
        _answerController.text,
      );

      setState(() {
        _feedback = result['feedback'];
        _answerController.clear();
      });
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Error: ${e.toString()}')),
      );
    } finally {
      setState(() => _isSubmitting = false);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
      appBar: AppBar(
        title: const Text('Exercise'),
      ),
      body: FutureBuilder<Exercise>(
        future: _exerciseFuture,
        builder: (context, snapshot) {
          if (snapshot.connectionState == ConnectionState.waiting) {
            return const Center(child: CircularProgressIndicator());
          }

          if (snapshot.hasError) {
            return Center(child: Text('Error: ${snapshot.error}'));
          }

          final exercise = snapshot.data!;

          return SingleChildScrollView(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  exercise.title,
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
                const SizedBox(height: 32),
                TextField(
                  controller: _answerController,
                  decoration: InputDecoration(
                    labelText: 'Your Answer',
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    hintText: 'Enter your answer...',
                  ),
                  minLines: 3,
                  maxLines: 5,
                ),
                const SizedBox(height: 16),
                SizedBox(
                  width: double.infinity,
                  height: 48,
                  child: ElevatedButton(
                    onPressed: _isSubmitting ? null : _submitAnswer,
                    child: _isSubmitting
                        ? const CircularProgressIndicator()
                        : const Text('Submit Answer'),
                  ),
                ),
                if (_feedback != null) ...[
                  const SizedBox(height: 24),
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: Colors.green.shade100,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: Colors.green),
                    ),
                    child: Text(_feedback!),
                  ),
                ],
              ],
            ),
          );
        },
      ),
    );
}
