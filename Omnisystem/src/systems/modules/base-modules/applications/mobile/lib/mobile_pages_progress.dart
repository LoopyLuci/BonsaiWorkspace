// PATHFINDER Mobile - Progress Page
// Detailed skill progress tracking and analytics

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/learning_provider.dart';

class ProgressPage extends StatefulWidget {
  const ProgressPage({Key? key}) : super(key: key);

  @override
  State<ProgressPage> createState() => _ProgressPageState();
}

class _ProgressPageState extends State<ProgressPage> {
  String _filterStatus = 'all'; // all, mastered, learning, struggling
  String _sortBy = 'recent'; // recent, mastery, time

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<LearningProvider>().fetchProgress();
    });
  }

  @override
  Widget build(BuildContext context) {
    final learningProvider = Provider.of<LearningProvider>(context);
    var skills = learningProvider.progress;

    // Apply filters
    if (_filterStatus != 'all') {
      skills = skills.where((skill) {
        final mastery = (skill['masteryPercent'] as double) ?? 0;
        switch (_filterStatus) {
          case 'mastered':
            return mastery >= 85;
          case 'learning':
            return mastery >= 30 && mastery < 85;
          case 'struggling':
            return mastery < 30;
          default:
            return true;
        }
      }).toList();
    }

    // Apply sorting
    skills.sort((a, b) {
      switch (_sortBy) {
        case 'mastery':
          return (b['masteryPercent'] as double).compareTo(a['masteryPercent'] as double);
        case 'time':
          return (b['lastUpdated'] as int? ?? 0).compareTo(a['lastUpdated'] as int? ?? 0);
        default:
          return 0;
      }
    });

    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Overall progress
            Card(
              color: const Color(0xFF4F46E5),
              child: Padding(
                padding: const EdgeInsets.all(20.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Overall Progress',
                      style: TextStyle(
                        color: Colors.white,
                        fontSize: 16,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          '${learningProvider.overallMastery.toStringAsFixed(1)}%',
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 32,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.end,
                          children: [
                            Text(
                              '${learningProvider.progress.where((s) => (s['masteryPercent'] as double) >= 85).length}/${learningProvider.progress.length}',
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 18,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                            const Text(
                              'Mastered',
                              style: TextStyle(
                                color: Color(0xFFE0E7FF),
                                fontSize: 12,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    const SizedBox(height: 16),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(4),
                      child: LinearProgressIndicator(
                        value: learningProvider.overallMastery / 100,
                        minHeight: 8,
                        backgroundColor: Colors.white24,
                        valueColor: const AlwaysStoppedAnimation<Color>(
                          Color(0xFF10B981),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 24),

            // Filters and sorting
            Row(
              children: [
                Expanded(
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: [
                        _buildFilterChip('All', 'all'),
                        const SizedBox(width: 8),
                        _buildFilterChip('Mastered', 'mastered'),
                        const SizedBox(width: 8),
                        _buildFilterChip('Learning', 'learning'),
                        const SizedBox(width: 8),
                        _buildFilterChip('Struggling', 'struggling'),
                      ],
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Sort dropdown
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                DropdownButton<String>(
                  value: _sortBy,
                  items: const [
                    DropdownMenuItem(value: 'recent', child: Text('Most Recent')),
                    DropdownMenuItem(value: 'mastery', child: Text('Highest Mastery')),
                    DropdownMenuItem(value: 'time', child: Text('Time Spent')),
                  ],
                  onChanged: (value) {
                    if (value != null) {
                      setState(() {
                        _sortBy = value;
                      });
                    }
                  },
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Skills list
            if (skills.isEmpty)
              Center(
                child: Padding(
                  padding: const EdgeInsets.all(32.0),
                  child: Column(
                    children: [
                      Icon(Icons.book, size: 48, color: Colors.grey[400]),
                      const SizedBox(height: 16),
                      Text(
                        'No skills in this category',
                        style: TextStyle(color: Colors.grey[600]),
                      ),
                    ],
                  ),
                ),
              )
            else
              ListView.builder(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: skills.length,
                itemBuilder: (context, index) {
                  final skill = skills[index];
                  final mastery = (skill['masteryPercent'] as double) ?? 0;
                  final exercisesAttempted = skill['exercisesAttempted'] as int? ?? 0;
                  final exercisesCorrect = skill['exercisesCorrect'] as int? ?? 0;

                  String status = 'Struggling';
                  Color statusColor = Colors.red;
                  if (mastery >= 85) {
                    status = 'Mastered';
                    statusColor = Colors.green;
                  } else if (mastery >= 30) {
                    status = 'Learning';
                    statusColor = Colors.blue;
                  }

                  return Card(
                    margin: const EdgeInsets.only(bottom: 12),
                    child: Padding(
                      padding: const EdgeInsets.all(16.0),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Expanded(
                                child: Text(
                                  skill['skillName'] ?? 'Unknown',
                                  style: const TextStyle(
                                    fontSize: 16,
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                              ),
                              Chip(
                                label: Text(
                                  status,
                                  style: const TextStyle(
                                    color: Colors.white,
                                    fontSize: 12,
                                  ),
                                ),
                                backgroundColor: statusColor,
                                padding: EdgeInsets.zero,
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Row(
                            children: [
                              Expanded(
                                child: ClipRRect(
                                  borderRadius: BorderRadius.circular(4),
                                  child: LinearProgressIndicator(
                                    value: mastery / 100,
                                    minHeight: 8,
                                    backgroundColor: const Color(0xFFE5E7EB),
                                    valueColor: AlwaysStoppedAnimation<Color>(
                                      mastery >= 85 ? Colors.green : mastery >= 50 ? Colors.blue : Colors.orange,
                                    ),
                                  ),
                                ),
                              ),
                              const SizedBox(width: 12),
                              Text(
                                '${mastery.toStringAsFixed(0)}%',
                                style: const TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text(
                                'Attempts: $exercisesAttempted',
                                style: TextStyle(
                                  fontSize: 12,
                                  color: Colors.grey[600],
                                ),
                              ),
                              Text(
                                'Correct: $exercisesCorrect',
                                style: TextStyle(
                                  fontSize: 12,
                                  color: Colors.grey[600],
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  );
                },
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildFilterChip(String label, String value) {
    final isSelected = _filterStatus == value;
    return FilterChip(
      label: Text(label),
      selected: isSelected,
      onSelected: (selected) {
        setState(() {
          _filterStatus = value;
        });
      },
      selectedColor: const Color(0xFF4F46E5),
      labelStyle: TextStyle(
        color: isSelected ? Colors.white : Colors.black,
        fontWeight: FontWeight.w500,
      ),
    );
  }
}
