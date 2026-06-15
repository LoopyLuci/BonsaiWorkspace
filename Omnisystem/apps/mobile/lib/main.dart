import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:go_router/go_router.dart';
import 'screens/login_screen.dart';
import 'screens/dashboard_screen.dart';
import 'screens/exercise_screen.dart';
import 'screens/classroom_screen.dart';
import 'screens/progress_screen.dart';
import 'screens/search_screen.dart';
import 'screens/analytics_screen.dart';
import 'screens/recommendations_screen.dart';
import 'providers/auth_provider.dart';
import 'providers/skill_provider.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) => MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => AuthProvider()),
        ChangeNotifierProvider(create: (_) => SkillProvider()),
      ],
      child: MaterialApp.router(
        title: 'PATHFINDER',
        theme: ThemeData(
          primarySwatch: Colors.blue,
          useMaterial3: true,
        ),
        routerConfig: _buildRouter(),
      ),
    );

  GoRouter _buildRouter() => GoRouter(
      redirect: (context, state) {
        final authProvider = context.read<AuthProvider>();
        final isLogin = state.matchedLocation == '/login';

        if (!authProvider.isAuthenticated && !isLogin) {
          return '/login';
        }

        if (authProvider.isAuthenticated && isLogin) {
          return '/dashboard';
        }

        return null;
      },
      routes: [
        GoRoute(
          path: '/login',
          builder: (context, state) => const LoginScreen(),
        ),
        GoRoute(
          path: '/dashboard',
          builder: (context, state) => const DashboardScreen(),
        ),
        GoRoute(
          path: '/exercise/:id',
          builder: (context, state) =>
              ExerciseScreen(exerciseId: state.pathParameters['id']!),
        ),
        GoRoute(
          path: '/classrooms',
          builder: (context, state) => const ClassroomScreen(),
        ),
        GoRoute(
          path: '/progress',
          builder: (context, state) => const ProgressScreen(),
        ),
        GoRoute(
          path: '/search',
          builder: (context, state) => const SearchScreen(),
        ),
        GoRoute(
          path: '/analytics',
          builder: (context, state) => const AnalyticsScreen(),
        ),
        GoRoute(
          path: '/recommendations',
          builder: (context, state) => const RecommendationsScreen(),
        ),
      ],
      initialLocation: '/dashboard',
    );
}
