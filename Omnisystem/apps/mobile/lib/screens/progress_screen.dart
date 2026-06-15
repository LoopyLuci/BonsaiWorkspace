import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/auth_provider.dart';
import '../providers/skill_provider.dart';

class ProgressScreen extends StatefulWidget {
  const ProgressScreen({Key? key}) : super(key: key);

  @override
  State<ProgressScreen> createState() => _ProgressScreenState();
}

class _ProgressScreenState extends State<ProgressScreen> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final authProvider = context.read<AuthProvider>();
      final skillProvider = context.read<SkillProvider>();
      if (authProvider.user != null) {
        skillProvider.fetchProgress(authProvider.user!.id);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final skillProvider = context.watch<SkillProvider>();

    return Scaffold(
      appBar: AppBar(
        title: const Text('Your Progress'),
      ),
      body: skillProvider.isLoading
          ? const Center(child: CircularProgressIndicator())
          : ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: skillProvider.progress.length,
        itemBuilder: (context, index) {
          final progress = skillProvider.progress[index];
          final percentage = (progress.pKnow * 100).toStringAsFixed(0);

          return Card(
            margin: const EdgeInsets.only(bottom: 16),
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        progress.skillId,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      Text('$percentage% Complete'),
                    ],
                  ),
                  const SizedBox(height: 12),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(8),
                    child: LinearProgressIndicator(
                      value: progress.pKnow,
                      minHeight: 8,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text('Attempts: ${progress.attempts}'),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}
