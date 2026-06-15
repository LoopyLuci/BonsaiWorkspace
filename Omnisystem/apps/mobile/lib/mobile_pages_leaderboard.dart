// PATHFINDER Mobile - Leaderboard Page
// Global rankings, competitive stats, and user positioning

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/learning_provider.dart';

class LeaderboardPage extends StatefulWidget {
  const LeaderboardPage({Key? key}) : super(key: key);

  @override
  State<LeaderboardPage> createState() => _LeaderboardPageState();
}

class _LeaderboardPageState extends State<LeaderboardPage> {
  String _timeRange = 'month';
  int _limit = 100;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<LearningProvider>().fetchLeaderboard(_limit);
    });
  }

  @override
  Widget build(BuildContext context) {
    final learningProvider = Provider.of<LearningProvider>(context);

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          // Header with user rank
          SliverAppBar(
            expandedHeight: 220,
            pinned: true,
            backgroundColor: Color(0xFF4F46E5),
            flexibleSpace: FlexibleSpaceBar(
              background: Container(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: [Color(0xFF4F46E5), Color(0xFF3B38CC)],
                  ),
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      'Global Leaderboard',
                      style: TextStyle(
                        color: Colors.white,
                        fontSize: 24,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    SizedBox(height: 24),
                    // User's position
                    Container(
                      padding: EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                      decoration: BoxDecoration(
                        color: Colors.white.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Column(
                        children: [
                          Text(
                            'Your Rank',
                            style: TextStyle(
                              color: Colors.white70,
                              fontSize: 12,
                            ),
                          ),
                          SizedBox(height: 4),
                          Text(
                            '#${learningProvider.userRank ?? '--'}',
                            style: TextStyle(
                              color: Colors.white,
                              fontSize: 28,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          SizedBox(height: 4),
                          Text(
                            'Top ${(100 - ((learningProvider.userPercentile ?? 50) * 100)).toStringAsFixed(0)}%',
                            style: TextStyle(
                              color: Colors.white70,
                              fontSize: 12,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),

          // Time range selector
          SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: Row(
                children: [
                  Expanded(
                    child: SegmentedButton<String>(
                      segments: const [
                        ButtonSegment(value: 'week', label: Text('Week')),
                        ButtonSegment(value: 'month', label: Text('Month')),
                        ButtonSegment(value: 'all', label: Text('All Time')),
                      ],
                      selected: {_timeRange},
                      onSelectionChanged: (newSelection) {
                        setState(() {
                          _timeRange = newSelection.first;
                        });
                        context.read<LearningProvider>().fetchLeaderboard(_limit);
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),

          // Top 3 Featured
          SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Text(
                'Top Learners',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
              ),
            ),
          ),

          SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: _buildTop3Featured(learningProvider),
            ),
          ),

          // Full leaderboard list
          SliverList(
            delegate: SliverChildBuilderDelegate(
              (context, index) {
                final leaderboard = learningProvider.leaderboard ?? [];
                if (index >= leaderboard.length) {
                  return SizedBox.shrink();
                }

                final entry = leaderboard[index];
                final rank = entry['rank'] as int? ?? (index + 1);
                final isCurrentUser = entry['userId'] == learningProvider.currentUserId;

                return Container(
                  margin: EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                  decoration: BoxDecoration(
                    color: isCurrentUser ? Color(0xFF4F46E5).withOpacity(0.1) : Colors.white,
                    border: Border.all(
                      color: isCurrentUser ? Color(0xFF4F46E5) : Colors.grey[300]!,
                      width: isCurrentUser ? 2 : 1,
                    ),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Padding(
                    padding: EdgeInsets.all(12),
                    child: Row(
                      children: [
                        // Rank medal or number
                        SizedBox(
                          width: 40,
                          child: Center(
                            child: rank <= 3
                                ? Text(
                                    _getMedal(rank),
                                    style: TextStyle(fontSize: 24),
                                  )
                                : Text(
                                    '#$rank',
                                    style: TextStyle(
                                      fontSize: 16,
                                      fontWeight: FontWeight.bold,
                                      color: Colors.grey[600],
                                    ),
                                  ),
                          ),
                        ),
                        SizedBox(width: 12),

                        // User info
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  Text(
                                    entry['name'] ?? 'Unknown',
                                    style: TextStyle(
                                      fontSize: 14,
                                      fontWeight: FontWeight.bold,
                                    ),
                                  ),
                                  if (isCurrentUser)
                                    Padding(
                                      padding: EdgeInsets.only(left: 8),
                                      child: Chip(
                                        label: Text('You', style: TextStyle(fontSize: 10)),
                                        backgroundColor: Color(0xFF4F46E5),
                                        labelStyle: TextStyle(color: Colors.white),
                                      ),
                                    ),
                                ],
                              ),
                              SizedBox(height: 4),
                              Row(
                                children: [
                                  Icon(Icons.star, size: 14, color: Colors.amber),
                                  SizedBox(width: 4),
                                  Text(
                                    '${entry['points'] ?? 0} pts',
                                    style: TextStyle(
                                      fontSize: 12,
                                      color: Colors.grey[600],
                                    ),
                                  ),
                                  SizedBox(width: 12),
                                  Icon(Icons.card_giftcard, size: 14, color: Colors.purple),
                                  SizedBox(width: 4),
                                  Text(
                                    '${entry['achievements'] ?? 0}',
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

                        // Mastery percentage
                        Container(
                          padding: EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                          decoration: BoxDecoration(
                            color: Colors.grey[100],
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Column(
                            children: [
                              Text(
                                '${(entry['mastery'] as double? ?? 0).toStringAsFixed(0)}%',
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.bold,
                                  color: Color(0xFF4F46E5),
                                ),
                              ),
                              Text(
                                'Mastery',
                                style: TextStyle(
                                  fontSize: 10,
                                  color: Colors.grey[600],
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              },
              childCount: learningProvider.leaderboard?.length ?? 0,
            ),
          ),

          // Load more button
          SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: ElevatedButton(
                onPressed: () {
                  setState(() {
                    _limit += 50;
                  });
                  context.read<LearningProvider>().fetchLeaderboard(_limit);
                },
                child: Text('Load More'),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTop3Featured(LearningProvider provider) {
    final leaderboard = provider.leaderboard ?? [];
    final topThree = leaderboard.take(3).toList();

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: List.generate(
          topThree.length,
          (index) {
            final entry = topThree[index];
            final rank = index + 1;

            return Container(
              width: 140,
              margin: EdgeInsets.only(right: 12),
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: _getPodiumGradient(rank),
                ),
                borderRadius: BorderRadius.circular(12),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withOpacity(0.1),
                    blurRadius: 8,
                  ),
                ],
              ),
              child: Padding(
                padding: EdgeInsets.all(12),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      _getMedal(rank),
                      style: TextStyle(fontSize: 32),
                    ),
                    SizedBox(height: 8),
                    Text(
                      '#$rank',
                      style: TextStyle(
                        color: Colors.white,
                        fontSize: 14,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    SizedBox(height: 8),
                    Text(
                      entry['name'] ?? 'Unknown',
                      textAlign: TextAlign.center,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    SizedBox(height: 8),
                    Container(
                      padding: EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: Colors.white.withOpacity(0.3),
                        borderRadius: BorderRadius.circular(6),
                      ),
                      child: Text(
                        '${entry['points'] ?? 0} pts',
                        style: TextStyle(
                          color: Colors.white,
                          fontSize: 11,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }

  List<Color> _getPodiumGradient(int rank) {
    switch (rank) {
      case 1:
        return [Colors.amber[700], Colors.amber[500]];
      case 2:
        return [Colors.grey[400], Colors.grey[300]];
      case 3:
        return [Colors.orange[700], Colors.orange[600]];
      default:
        return [Colors.blue, Colors.blue[300]];
    }
  }

  String _getMedal(int rank) {
    switch (rank) {
      case 1:
        return '🥇';
      case 2:
        return '🥈';
      case 3:
        return '🥉';
      default:
        return '';
    }
  }
}
