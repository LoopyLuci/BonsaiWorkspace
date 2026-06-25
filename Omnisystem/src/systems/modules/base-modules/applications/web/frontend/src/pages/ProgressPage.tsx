import { useQuery } from '@tanstack/react-query'
import { progressAPI } from '../api'
import { useAuthStore } from '../stores/authStore'

export default function ProgressPage() {
  const { user } = useAuthStore()
  const { data: progress, isLoading } = useQuery({
    queryKey: ['progress', user?.id],
    queryFn: () => progressAPI.list(user?.id || ''),
    enabled: !!user?.id,
  })

  if (isLoading) return <div className="p-8">Loading...</div>

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto">
        <h1 className="text-3xl font-bold mb-8">Your Progress</h1>

        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="w-full">
            <thead className="bg-gray-100">
              <tr>
                <th className="px-6 py-3 text-left text-gray-900 font-bold">Skill</th>
                <th className="px-6 py-3 text-left text-gray-900 font-bold">Mastery</th>
                <th className="px-6 py-3 text-left text-gray-900 font-bold">Attempts</th>
              </tr>
            </thead>
            <tbody>
              {progress?.data?.map((p: any) => (
                <tr key={p.skill_id} className="border-t">
                  <td className="px-6 py-4">{p.skill_id}</td>
                  <td className="px-6 py-4">
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-blue-600 h-2 rounded-full"
                        style={{ width: `${p.p_know * 100}%` }}
                      ></div>
                    </div>
                    <p className="text-sm mt-1">{Math.round(p.p_know * 100)}%</p>
                  </td>
                  <td className="px-6 py-4">{p.attempts}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
