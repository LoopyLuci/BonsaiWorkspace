import { useQuery } from '@tanstack/react-query'
import { skillsAPI, progressAPI } from '../api'
import { useAuthStore } from '../stores/authStore'

export default function DashboardPage() {
  const { user } = useAuthStore()
  const { data: skills, isLoading: skillsLoading } = useQuery({
    queryKey: ['skills'],
    queryFn: () => skillsAPI.list(),
  })

  const { data: progress } = useQuery({
    queryKey: ['progress', user?.id],
    queryFn: () => progressAPI.list(user?.id || ''),
    enabled: !!user?.id,
  })

  if (skillsLoading) return <div className="p-8">Loading...</div>

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto">
        <h1 className="text-3xl font-bold mb-8">Welcome, {user?.name}!</h1>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {skills?.data?.map((skill: any) => (
            <div key={skill.id} className="bg-white rounded-lg shadow p-6">
              <h3 className="text-xl font-bold mb-2">{skill.name}</h3>
              <p className="text-gray-600 mb-4">
                Difficulty: <span className="capitalize">{skill.difficulty}</span>
              </p>
              <div className="w-full bg-gray-200 rounded-full h-2">
                <div
                  className="bg-blue-600 h-2 rounded-full"
                  style={{
                    width: `${
                      progress?.data?.find((p: any) => p.skill_id === skill.id)
                        ?.p_know * 100 || 0
                    }%`,
                  }}
                ></div>
              </div>
              <p className="text-sm text-gray-600 mt-2">
                {progress?.data?.find((p: any) => p.skill_id === skill.id)
                  ?.p_know
                  ? `${Math.round(
                      progress.data.find((p: any) => p.skill_id === skill.id)
                        .p_know * 100
                    )}% mastered`
                  : 'Not started'}
              </p>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
