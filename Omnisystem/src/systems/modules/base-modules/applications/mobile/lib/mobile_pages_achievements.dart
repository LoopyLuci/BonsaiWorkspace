// PATHFINDER Mobile - Achievements Page
// Badge collection, XP tracking, and gamification display

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/learning_provider.dart';

class AchievementsPage extends StatefulWidget {
  const AchievementsPage({Key? key}) : super(key: key);

  @override
  State<AchievementsPage> createState() => _AchievementsPageState();
}

class _AchievementsPageState extends State<AchievementsPage> with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final String _selectedCategory = 'all';

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 4, vsync: this);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<LearningProvider>().fetchAchievements();
    });
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final learningProvider = Provider.of<LearningProvider>(context);

    return SafeArea(
      child: DefaultTabController(
        length: 4,
        child: NestedScrollView(
          headerSliverBuilder: (context, innerBoxIsScrolled) => [
              SliverAppBar(
                expandedHeight: 200,
                pinned: true,
                flexibleSpace: FlexibleSpaceBar(
                  background: Container(
                    decoration: BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                        colors: [Color(0xFF4F46E5), Color(0xFF3B38CC)],
                      ),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.all(20.0),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisAlignment: MainAxisAlignment.end,
                        children: [
                          Text(
                            'Achievements',
                            style: Theme.of(context).textTheme.headlineLarge?.copyWith(
                              color: Colors.white,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          SizedBox(height: 12),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              _buildStatCard('${learningProvider.totalPoints}', 'Points', Colors.orange),
                              _buildStatCard('${learningProvider.userLevel}', 'Level', Colors.blue),
                              _buildStatCard('${learningProvider.unlockedBadgesCount}', 'Badges', Colors.green),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
              SliverPersistentHeader(
                pinned: true,
                delegate: _SliverAppBarDelegate(
                  TabBar(
                    controller: _tabController,
                    labelColor: Color(0xFF4F46E5),
                    unselectedLabelColor: Colors.grey,
                    indicatorColor: Color(0xFF4F46E5),
                    tabs: const [
                      Tab(text: 'All'),
                      Tab(text: 'Unlocked'),
                      Tab(text: 'Locked'),
                      Tab(text: 'Rare'),
                    ],
                  ),
                ),
              ),
            ],
          body: TabBarView(
            controller: _tabController,
            children: [
              _buildAchievementsList(learningProvider, null),
              _buildAchievementsList(learningProvider, 'unlocked'),
              _buildAchievementsList(learningProvider, 'locked'),
              _buildAchievementsList(learningProvider, 'rare'),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStatCard(String value, String label, Color color) => Expanded(
      child: Container(
        margin: EdgeInsets.symmetric(horizontal: 4),
        padding: EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: Colors.white.withOpacity(0.2),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          children: [
            Text(
              value,
              style: TextStyle(
                color: Colors.white,
                fontSize: 20,
                fontWeight: FontWeight.bold,
              ),
            ),
            Text(
              label,
              style: TextStyle(
                color: Colors.white70,
                fontSize: 12,
              ),
            ),
          ],
        ),
      ),
    );

  Widget _buildAchievementsList(LearningProvider provider, String? filter) {
    var achievements = provider.achievements ?? [];

    if (filter == 'unlocked') {
      achievements = achievements.where((a) => a['unlocked'] == true).toList();
    } else if (filter == 'locked') {
      achievements = achievements.where((a) => a['unlocked'] != true).toList();
    } else if (filter == 'rare') {
      achievements = achievements.where((a) {
        final rarity = a['rarity'] as String?;
        return rarity == 'epic' || rarity == 'legendary';
      }).toList();
    }

    if (achievements.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.card_giftcard, size: 48, color: Colors.grey[300]),
            SizedBox(height: 16),
            Text('No achievements yet', style: TextStyle(color: Colors.grey[600])),
          ],
        ),
      );
    }

    return GridView.builder(
      padding: EdgeInsets.all(16),
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 2,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
        childAspectRatio: 0.85,
      ),
      itemCount: achievements.length,
      itemBuilder: (context, index) {
        final achievement = achievements[index];
        final isUnlocked = achievement['unlocked'] == true;
        final rarity = achievement['rarity'] as String? ?? 'common';
        final rarityColor = _getRarityColor(rarity);

        return GestureDetector(
          onTap: () => _showAchievementDetails(context, achievement),
          child: Container(
            decoration: BoxDecoration(
              color: isUnlocked ? Colors.white : Colors.grey[200],
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: rarityColor.withOpacity(0.5),
                width: 2,
              ),
              boxShadow: isUnlocked
                  ? [BoxShadow(color: rarityColor.withOpacity(0.3), blurRadius: 8)]
                  : [],
            ),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                // Badge icon or lock
                if (isUnlocked)
                  Icon(
                    Icons.card_giftcard,
                    size: 48,
                    color: rarityColor,
                  )
                else
                  Icon(
                    Icons.lock,
                    size: 48,
                    color: Colors.grey[400],
                  ),
                SizedBox(height: 8),
                Text(
                  achievement['name'] ?? 'Unknown',
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    color: isUnlocked ? Colors.black : Colors.grey[600],
                  ),
                ),
                SizedBox(height: 4),
                if (isUnlocked)
                  Chip(
                    label: Text(
                      '${achievement['points'] ?? 0} XP',
                      style: TextStyle(fontSize: 10, color: Colors.white),
                    ),
                    backgroundColor: rarityColor,
                    padding: EdgeInsets.symmetric(horizontal: 8),
                  )
                else
                  Text(
                    'Locked',
                    style: TextStyle(
                      fontSize: 10,
                      color: Colors.grey[500],
                      fontStyle: FontStyle.italic,
                    ),
                  ),
              ],
            ),
          ),
        );
      },
    );
  }

  Color _getRarityColor(String rarity) {
    switch (rarity) {
      case 'common':
        return Colors.grey;
      case 'uncommon':
        return Colors.green;
      case 'rare':
        return Colors.blue;
      case 'epic':
        return Colors.purple;
      case 'legendary':
        return Colors.amber;
      default:
        return Colors.grey;
    }
  }

  void _showAchievementDetails(BuildContext context, Map achievement) {
    showModalBottomSheet(
      context: context,
      builder: (context) => Container(
        padding: EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Center(
              child: Icon(
                Icons.card_giftcard,
                size: 64,
                color: _getRarityColor(achievement['rarity'] ?? 'common'),
              ),
            ),
            SizedBox(height: 16),
            Text(
              achievement['name'] ?? 'Unknown',
              style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
            ),
            SizedBox(height: 8),
            Text(
              achievement['description'] ?? '',
              style: TextStyle(color: Colors.grey[600]),
            ),
            SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                Column(
                  children: [
                    Text('Points', style: TextStyle(color: Colors.grey[600])),
                    Text(
                      '${achievement['points'] ?? 0}',
                      style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
                    ),
                  ],
                ),
                Column(
                  children: [
                    Text('Rarity', style: TextStyle(color: Colors.grey[600])),
                    Chip(
                      label: Text(achievement['rarity'] ?? 'common'),
                      backgroundColor: _getRarityColor(achievement['rarity'] ?? 'common').withOpacity(0.2),
                    ),
                  ],
                ),
              ],
            ),
            SizedBox(height: 16),
            if (achievement['unlocked'] == true)
              Text(
                'Unlocked on ${_formatDate(achievement['unlockedAt'])}',
                style: TextStyle(color: Colors.green, fontSize: 12),
              )
            else
              Text(
                'Requirement: ${achievement['requirement'] ?? 'Unknown'}',
                style: TextStyle(color: Colors.grey[600], fontSize: 12),
              ),
          ],
        ),
      ),
    );
  }

  String _formatDate(dynamic date) {
    if (date == null) return 'Unknown';
    try {
      final dt = DateTime.parse(date.toString());
      return '${dt.month}/${dt.day}/${dt.year}';
    } catch (e) {
      return 'Unknown';
    }
  }
}

class _SliverAppBarDelegate extends SliverPersistentHeaderDelegate {

  _SliverAppBarDelegate(this._tabBar);
  final TabBar _tabBar;

  @override
  double get minExtent => _tabBar.preferredSize.height;

  @override
  double get maxExtent => _tabBar.preferredSize.height;

  @override
  Widget build(BuildContext context, double shrinkOffset, bool overlapsContent) => Container(
      color: Colors.white,
      child: _tabBar,
    );

  @override
  bool shouldRebuild(_SliverAppBarDelegate oldDelegate) => false;
}
