class Skill {

  Skill({
    required this.id,
    required this.name,
    required this.difficulty,
    required this.prerequisites,
  });

  factory Skill.fromJson(Map<String, dynamic> json) {
    return Skill(
      id: json['id'] as String,
      name: json['name'] as String,
      difficulty: json['difficulty'] as String,
      prerequisites: List<String>.from(json['prerequisites'] as List? ?? []),
    );
  }
  final String id;
  final String name;
  final String difficulty;
  final List<String> prerequisites;
}

class Exercise {

  Exercise({
    required this.id,
    required this.skillId,
    required this.title,
    required this.problemType,
  });

  factory Exercise.fromJson(Map<String, dynamic> json) {
    return Exercise(
      id: json['id'] as String,
      skillId: json['skill_id'] as String,
      title: json['title'] as String,
      problemType: json['problem_type'] as String,
    );
  }
  final String id;
  final String skillId;
  final String title;
  final String problemType;
}

class SkillProgress {

  SkillProgress({
    required this.userId,
    required this.skillId,
    required this.pKnow,
    required this.attempts,
  });

  factory SkillProgress.fromJson(Map<String, dynamic> json) {
    return SkillProgress(
      userId: json['user_id'] as String,
      skillId: json['skill_id'] as String,
      pKnow: (json['p_know'] as num).toDouble(),
      attempts: json['attempts'] as int,
    );
  }
  final String userId;
  final String skillId;
  final double pKnow;
  final int attempts;
}
