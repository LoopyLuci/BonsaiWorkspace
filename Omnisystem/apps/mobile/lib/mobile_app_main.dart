// PATHFINDER Mobile App - Main Entry Point
// Flutter app for iOS and Android with offline-first architecture

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:hive_flutter/hive_flutter.dart';
import 'package:intl/intl.dart';
import 'pages/login_page.dart';
import 'pages/dashboard_page.dart';
import 'pages/exercise_page.dart';
import 'pages/progress_page.dart';
import 'pages/achievements_page.dart';
import 'pages/leaderboard_page.dart';
import 'pages/settings_page.dart';
import 'services/api_service.dart';
import 'services/offline_sync_service.dart';
import 'services/local_storage_service.dart';
import 'providers/auth_provider.dart';
import 'providers/learning_provider.dart';
import 'providers/sync_provider.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize Hive for local storage
  await Hive.initFlutter();
  await Hive.openBox('user_data');
  await Hive.openBox('exercises');
  await Hive.openBox('progress');
  await Hive.openBox('sync_queue');

  // Initialize services
  final localStorageService = LocalStorageService();
  final apiService = ApiService();
  final offlineSyncService = OfflineSyncService(apiService, localStorageService);

  runApp(
    MultiProvider(
      providers: [
        Provider(create: (_) => localStorageService),
        Provider(create: (_) => apiService),
        Provider(create: (_) => offlineSyncService),
        ChangeNotifierProvider(
          create: (_) => AuthProvider(localStorageService, apiService),
        ),
        ChangeNotifierProvider(
          create: (_) => LearningProvider(apiService, localStorageService),
        ),
        ChangeNotifierProvider(
          create: (_) => SyncProvider(offlineSyncService),
        ),
      ],
      child: const PathfinderApp(),
    ),
  );
}

class PathfinderApp extends StatelessWidget {
  const PathfinderApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) => MaterialApp(
      title: 'PATHFINDER',
      theme: ThemeData(
        primarySwatch: Colors.indigo,
        useMaterial3: true,
        brightness: Brightness.light,
        appBarTheme: const AppBarTheme(
          elevation: 0,
          centerTitle: true,
          backgroundColor: Color(0xFF4F46E5),
          foregroundColor: Colors.white,
        ),
        floatingActionButtonTheme: const FloatingActionButtonThemeData(
          backgroundColor: Color(0xFF4F46E5),
        ),
        inputDecorationTheme: InputDecorationTheme(
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(8),
          ),
          filled: true,
          fillColor: const Color(0xFFF3F4F6),
          contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        ),
        elevatedButtonTheme: ElevatedButtonThemeData(
          style: ElevatedButton.styleFrom(
            padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(8),
            ),
          ),
        ),
      ),
      darkTheme: ThemeData(
        brightness: Brightness.dark,
        useMaterial3: true,
        primarySwatch: Colors.indigo,
      ),
      home: const AppRouter(),
      debugShowCheckedModeBanner: false,
    );
}

class AppRouter extends StatelessWidget {
  const AppRouter({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final authProvider = Provider.of<AuthProvider>(context);

    if (authProvider.isLoading) {
      return const Scaffold(
        body: Center(
          child: CircularProgressIndicator(),
        ),
      );
    }

    if (authProvider.isAuthenticated) {
      return const MainApp();
    } else {
      return const LoginPage();
    }
  }
}

class MainApp extends StatefulWidget {
  const MainApp({Key? key}) : super(key: key);

  @override
  State<MainApp> createState() => _MainAppState();
}

class _MainAppState extends State<MainApp> {
  int _selectedIndex = 0;
  late List<Widget> _pages;

  @override
  void initState() {
    super.initState();
    _pages = [
      const DashboardPage(),
      const ExercisePage(),
      const ProgressPage(),
      const AchievementsPage(),
      const LeaderboardPage(),
      const SettingsPage(),
    ];

    // Start background sync
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<SyncProvider>().startBackgroundSync();
    });
  }

  @override
  Widget build(BuildContext context) {
    final syncProvider = Provider.of<SyncProvider>(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('PATHFINDER'),
        actions: [
          // Sync indicator
          if (syncProvider.isSyncing)
            const Padding(
              padding: EdgeInsets.all(16.0),
              child: SizedBox(
                width: 24,
                height: 24,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                ),
              ),
            ),
          // Offline indicator
          if (!syncProvider.isOnline)
            const Padding(
              padding: EdgeInsets.all(16.0),
              child: Tooltip(
                message: 'Offline Mode',
                child: Icon(Icons.cloud_off),
              ),
            ),
        ],
      ),
      body: IndexedStack(
        index: _selectedIndex,
        children: _pages,
      ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _selectedIndex,
        type: BottomNavigationBarType.fixed,
        items: const [
          BottomNavigationBarItem(
            icon: Icon(Icons.home),
            label: 'Home',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.book),
            label: 'Learn',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.trending_up),
            label: 'Progress',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.trophy),
            label: 'Achievements',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.leaderboard),
            label: 'Leaderboard',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.settings),
            label: 'Settings',
          ),
        ],
        onTap: (index) {
          setState(() {
            _selectedIndex = index;
          });
        },
      ),
    );
  }
}
