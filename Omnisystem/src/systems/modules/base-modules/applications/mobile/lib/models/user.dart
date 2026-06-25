class User {

  User({
    required this.id,
    required this.email,
    required this.name,
    required this.role,
  });

  factory User.fromJson(Map<String, dynamic> json) {
    return User(
      id: json['id'] as String,
      email: json['email'] as String,
      name: json['name'] as String,
      role: json['role'] as String? ?? 'student',
    );
  }
  final String id;
  final String email;
  final String name;
  final String role;

  Map<String, dynamic> toJson() => {
    'id': id,
    'email': email,
    'name': name,
    'role': role,
  };
}

class AuthResponse {

  AuthResponse({
    required this.userId,
    required this.token,
    required this.email,
  });

  factory AuthResponse.fromJson(Map<String, dynamic> json) {
    return AuthResponse(
      userId: json['user_id'] as String,
      token: json['token'] as String,
      email: json['email'] as String,
    );
  }
  final String userId;
  final String token;
  final String email;
}
