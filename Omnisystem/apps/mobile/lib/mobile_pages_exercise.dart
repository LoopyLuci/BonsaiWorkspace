// PATHFINDER Mobile - Exercise Page
// Interactive exercise with offline support and instant feedback

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/learning_provider.dart';
import '../providers/sync_provider.dart';
import '../services/offline_sync_service.dart';

class ExercisePage extends StatefulWidget {
  const ExercisePage({Key? key}) : super(key: key);

  @override
  State<ExercisePage> createState() => _ExercisePageState();
}

class _ExercisePageState extends State<ExercisePage> {
  int? _selectedIndex;
  bool _showExplanation = false;
  int _secondsRemaining = 300; // 5 minutes
  late DateTime _startTime;

  @override
  void initState() {
    super.initState();
    _startTime = DateTime.now();
    _startTimer();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<LearningProvider>().fetchExercises();
    });
  }

  void _startTimer() {
    Future.delayed(const Duration(seconds: 1), () {
      if (mounted && !_showExplanation) {
        setState(() {
          _secondsRemaining--;
          if (_secondsRemaining > 0) {
            _startTimer();
          }
        });
      }
    });
  }

  String _formatTime(int seconds) {
    final minutes = seconds ~/ 60;
    final secs = seconds % 60;
    return '${minutes.toString().padLeft(2, '0')}:${secs.toString().padLeft(2, '0')}';
  }

  void _submitAnswer(Map exercise, int selectedIndex) async {
    final learningProvider = context.read<LearningProvider>();
    final syncService = context.read<OfflineSyncService>();

    final timeTaken = DateTime.now().difference(_startTime).inSeconds;

    await learningProvider.submitAttempt(
      exerciseId: exercise['exerciseId'],
      skillId: exercise['skillId'],
      selectedIndex: selectedIndex,
      timeTakenSeconds: timeTaken,
      syncService: syncService,
    );

    setState(() {
      _showExplanation = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    final learningProvider = Provider.of<LearningProvider>(context);
    final exercises = learningProvider.exercises;

    if (learningProvider.isLoading) {
      return const Scaffold(
        body: Center(
          child: CircularProgressIndicator(),
        ),
      );
    }

    if (exercises.isEmpty) {
      return Scaffold(
        body: SafeArea(
          child: Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(Icons.school, size: 64, color: Colors.grey[400]),
                const SizedBox(height: 16),
                const Text(
                  'No exercises available',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 8),
                Text(
                  'Check back later for more challenges',
                  style: TextStyle(color: Colors.grey[600]),
                ),
              ],
            ),
          ),
        ),
      );
    }

    final exercise = exercises[0]; // Get first exercise
    final isCorrect = exercise['correctIndex'] == _selectedIndex;

    return Scaffold(
      appBar: AppBar(
        title: Text(exercise['skillName'] ?? 'Exercise'),
        actions: [
          Padding(
            padding: const EdgeInsets.all(16.0),
            child: Center(
              child: Text(
                _formatTime(_secondsRemaining),
                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.bold,
                  color: Colors.white,
                ),
              ),
            ),
          ),
        ],
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Difficulty indicator
              Chip(
                label: Text('Difficulty: ${exercise['difficulty']}'),
                backgroundColor: _getDifficultyColor(exercise['difficulty']),
                labelStyle: const TextStyle(color: Colors.white),
              ),
              const SizedBox(height: 24),

              // Question
              Text(
                exercise['question'] ?? '',
                style: const TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.bold,
                  height: 1.5,
                ),
              ),
              const SizedBox(height: 32),

              // Answer options
              if (!_showExplanation)
                ...(exercise['options'] as List<String>? ?? [])
                    .asMap()
                    .entries
                    .map((e) => _buildOptionButton(
                          index: e.key,
                          text: e.value,
                          onPressed: () {
                            setState(() {
                              _selectedIndex = e.key;
                            });
                          },
                          isSelected: _selectedIndex == e.key,
                          enabled: true,
                        )),

              // Show answer after submission
              if (_showExplanation) ...[
                ...(exercise['options'] as List<String>? ?? [])
                    .asMap()
                    .entries
                    .map((e) => _buildOptionButton(
                          index: e.key,
                          text: e.value,
                          onPressed: null,
                          isSelected: _selectedIndex == e.key,
                          isCorrect: e.key == exercise['correctIndex'],
                          enabled: false,
                        )),
                const SizedBox(height: 24),

                // Explanation
                Card(
                  color: const Color(0xFFF0F9FF),
                  child: Padding(
                    padding: const EdgeInsets.all(16.0),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          isCorrect ? '✓ Correct!' : '✗ Incorrect',
                          style: TextStyle(
                            fontSize: 18,
                            fontWeight: FontWeight.bold,
                            color: isCorrect ? Colors.green : Colors.red,
                          ),
                        ),
                        const SizedBox(height: 12),
                        Text(
                          exercise['explanation'] ?? 'No explanation available',
                          style: const TextStyle(
                            fontSize: 14,
                            height: 1.6,
                            color: Color(0xFF1E40AF),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 24),

                // Next button
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton(
                    onPressed: () {
                      setState(() {
                        _selectedIndex = null;
                        _showExplanation = false;
                        _secondsRemaining = 300;
                      });
                      // In real app, fetch next exercise
                    },
                    style: ElevatedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 12),
                      backgroundColor: const Color(0xFF4F46E5),
                      foregroundColor: Colors.white,
                    ),
                    child: const Text('Next Exercise'),
                  ),
                ),
              ] else ...[
                const SizedBox(height: 32),
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton(
                    onPressed: _selectedIndex != null
                        ? () => _submitAnswer(exercise, _selectedIndex!)
                        : null,
                    style: ElevatedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 12),
                      backgroundColor: const Color(0xFF4F46E5),
                      foregroundColor: Colors.white,
                    ),
                    child: const Text('Submit Answer'),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildOptionButton({
    required int index,
    required String text,
    required VoidCallback? onPressed,
    required bool isSelected,
    bool isCorrect = false,
    required bool enabled,
  }) {
    Color backgroundColor;
    Color borderColor;
    Color textColor;

    if (!enabled) {
      if (isCorrect) {
        backgroundColor = Colors.green.shade100;
        borderColor = Colors.green;
        textColor = Colors.green.shade900;
      } else if (isSelected && !isCorrect) {
        backgroundColor = Colors.red.shade100;
        borderColor = Colors.red;
        textColor = Colors.red.shade900;
      } else {
        backgroundColor = Colors.grey.shade100;
        borderColor = Colors.grey.shade300;
        textColor = Colors.grey.shade700;
      }
    } else {
      if (isSelected) {
        backgroundColor = const Color(0xFF4F46E5);
        borderColor = const Color(0xFF4F46E5);
        textColor = Colors.white;
      } else {
        backgroundColor = const Color(0xFFF3F4F6);
        borderColor = const Color(0xFFD1D5DB);
        textColor = Colors.black;
      }
    }

    return Padding(
      padding: const EdgeInsets.only(bottom: 12.0),
      child: Material(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: enabled ? onPressed : null,
          borderRadius: BorderRadius.circular(8),
          child: Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              border: Border.all(color: borderColor, width: 2),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              children: [
                Container(
                  width: 32,
                  height: 32,
                  decoration: BoxDecoration(
                    border: Border.all(color: textColor, width: 2),
                    borderRadius: BorderRadius.circular(4),
                    color: isSelected ? textColor : Colors.transparent,
                  ),
                  child: Center(
                    child: Text(
                      String.fromCharCode(65 + index),
                      style: TextStyle(
                        color: isSelected ? backgroundColor : textColor,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Text(
                    text,
                    style: TextStyle(
                      color: textColor,
                      fontSize: 14,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                if (!enabled && isCorrect)
                  Icon(Icons.check_circle, color: Colors.green.shade700),
                if (!enabled && isSelected && !isCorrect)
                  Icon(Icons.cancel, color: Colors.red.shade700),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Color _getDifficultyColor(int difficulty) {
    switch (difficulty) {
      case 1:
        return Colors.green;
      case 2:
        return Colors.blue;
      case 3:
        return Colors.orange;
      default:
        return Colors.red;
    }
  }

  @override
  void dispose() {
    super.dispose();
  }
}
