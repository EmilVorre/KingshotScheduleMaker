import { useEffect, useState, useMemo } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api, type AccountStats } from '../api/client'

function parseTimeToMinutes(timeStr: string): number {
  const parts = timeStr.split(':')
  if (parts.length !== 2) return 0
  return (parseInt(parts[0], 10) || 0) * 60 + (parseInt(parts[1], 10) || 0)
}

function sortTimeSlots(
  map: Record<string, { requests: number }> | undefined,
  startTime: string
): Record<string, { requests: number }> {
  if (!map) return {}
  const entries = Object.entries(map)
  const startMinutes = parseTimeToMinutes(startTime)
  entries.sort((a, b) => {
    const adjA = parseTimeToMinutes(a[0]) < startMinutes ? parseTimeToMinutes(a[0]) + 1440 : parseTimeToMinutes(a[0])
    const adjB = parseTimeToMinutes(b[0]) < startMinutes ? parseTimeToMinutes(b[0]) + 1440 : parseTimeToMinutes(b[0])
    return adjA - adjB
  })
  return Object.fromEntries(entries)
}

function getTotal(d: { construction_requests?: number; research_requests?: number; troops_requests?: number; requests?: number }): number {
  if ('requests' in d && typeof d.requests === 'number') return d.requests
  return (d.construction_requests ?? 0) + (d.research_requests ?? 0) + (d.troops_requests ?? 0)
}

export default function StatsPage() {
  const { accountName, server } = useParams<{ accountName: string; server: string }>()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [stats, setStats] = useState<AccountStats | null>(null)

  useEffect(() => {
    if (!accountName || !server) return
    setLoading(true)
    setError(null)
    api.getStats(accountName, parseInt(server, 10)).then(({ ok, data, error: err }) => {
      if (ok && data) setStats(data as AccountStats)
      else setError(err || 'Failed to load statistics')
      setLoading(false)
    })
  }, [accountName, server])

  const sortedAlliances = useMemo(() => {
    if (!stats?.alliance_counts) return {}
    const entries = Object.entries(stats.alliance_counts)
    entries.sort((a, b) => getTotal(b[1]) - getTotal(a[1]))
    return Object.fromEntries(entries)
  }, [stats])

  const sortedTimeSlots = useMemo(() => {
    if (!stats?.time_slot_popularity) return {}
    const entries = Object.entries(stats.time_slot_popularity)
    entries.sort((a, b) => a[0].localeCompare(b[0]))
    return Object.fromEntries(entries)
  }, [stats])

  const sortedConstruction = useMemo(
    () => sortTimeSlots(stats?.construction_time_slot_popularity, stats?.construction_start_time || '00:00'),
    [stats]
  )
  const sortedResearch = useMemo(
    () => sortTimeSlots(stats?.research_time_slot_popularity, stats?.research_start_time || '00:00'),
    [stats]
  )
  const sortedTroops = useMemo(
    () => sortTimeSlots(stats?.troops_time_slot_popularity, stats?.troops_start_time || '00:00'),
    [stats]
  )

  if (!accountName || !server) {
    return (
      <div className="container mx-auto px-4 py-8">
        <p className="text-red-400">Invalid URL</p>
        <Link to="/" className="text-blue-400 mt-4 inline-block">← Home</Link>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-7xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-chart-bar mr-3"></i>Statistics
        </h1>
        <p className="text-gray-400">Alliance Requests & Time Slot Popularity</p>
      </header>

      {loading && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
          <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
          <p className="text-xl text-gray-400">Loading statistics...</p>
        </div>
      )}
      {error && (
        <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
          <i className="fas fa-exclamation-circle mr-2"></i>{error}
        </div>
      )}
      {!loading && !error && stats && (
        <div className="space-y-8">
          {Object.keys(sortedAlliances).length > 0 && (
            <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
              <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                <i className="fas fa-users text-blue-400 mr-3"></i>Alliance Request Counts
              </h2>
              <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
                {Object.entries(sortedAlliances).map(([alliance, data]) => (
                  <div
                    key={alliance}
                    className="bg-gray-700/50 rounded-xl p-5 border-2 border-gray-600 hover:border-blue-500 hover:shadow-lg transition-all"
                  >
                    <h3 className="text-xl font-bold text-white mb-4">{alliance || '(No Alliance)'}</h3>
                    <div className="space-y-2">
                      <div className="flex justify-between items-center">
                        <span className="text-gray-300 flex items-center"><i className="fas fa-hammer text-orange-400 mr-2"></i>Construction:</span>
                        <strong className="text-orange-400 text-lg">{data.construction_requests}</strong>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-gray-300 flex items-center"><i className="fas fa-flask text-blue-400 mr-2"></i>Research:</span>
                        <strong className="text-blue-400 text-lg">{data.research_requests}</strong>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-gray-300 flex items-center"><i className="fas fa-users text-green-400 mr-2"></i>Troops:</span>
                        <strong className="text-green-400 text-lg">{data.troops_requests}</strong>
                      </div>
                      <div className="flex justify-between items-center pt-3 border-t-2 border-gray-600 mt-3">
                        <span className="font-bold text-gray-200">Total:</span>
                        <strong className="text-blue-400 text-xl">{getTotal(data)}</strong>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {Object.keys(sortedConstruction).length > 0 && (
            <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
              <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                <i className="fas fa-hammer text-orange-400 mr-3"></i>Construction Day Time Slot Popularity
              </h2>
              <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                {Object.entries(sortedConstruction).map(([time, data]) => (
                  <div key={time} className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-orange-500 hover:shadow-md transition-all text-center">
                    <div className="font-bold text-orange-400 text-sm mb-1">{time}</div>
                    <div className="text-xl font-bold text-white">{data.requests}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {Object.keys(sortedResearch).length > 0 && (
            <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
              <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                <i className="fas fa-flask text-blue-400 mr-3"></i>Research Day Time Slot Popularity
              </h2>
              <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                {Object.entries(sortedResearch).map(([time, data]) => (
                  <div key={time} className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-blue-500 hover:shadow-md transition-all text-center">
                    <div className="font-bold text-blue-400 text-sm mb-1">{time}</div>
                    <div className="text-xl font-bold text-white">{data.requests}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {Object.keys(sortedTroops).length > 0 && (
            <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
              <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                <i className="fas fa-users text-green-400 mr-3"></i>Troops Training Day Time Slot Popularity
              </h2>
              <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                {Object.entries(sortedTroops).map(([time, data]) => (
                  <div key={time} className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-green-500 hover:shadow-md transition-all text-center">
                    <div className="font-bold text-green-400 text-sm mb-1">{time}</div>
                    <div className="text-xl font-bold text-white">{data.requests}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {!stats.construction_time_slot_popularity && Object.keys(sortedTimeSlots).length > 0 && (
            <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
              <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                <i className="fas fa-clock text-blue-400 mr-3"></i>Time Slot Popularity
              </h2>
              <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                {Object.entries(sortedTimeSlots).map(([time, data]) => (
                  <div key={time} className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-blue-500 hover:shadow-md transition-all text-center">
                    <div className="font-bold text-blue-400 text-sm mb-1">{time}</div>
                    <div className="space-y-1 text-xs">
                      <div className="flex justify-between"><span className="text-orange-400"><i className="fas fa-hammer"></i></span><strong className="text-gray-200">{data.construction_requests}</strong></div>
                      <div className="flex justify-between"><span className="text-blue-400"><i className="fas fa-flask"></i></span><strong className="text-gray-200">{data.research_requests}</strong></div>
                      <div className="flex justify-between"><span className="text-green-400"><i className="fas fa-users"></i></span><strong className="text-gray-200">{data.troops_requests}</strong></div>
                    </div>
                    <div className="mt-1 pt-1 border-t border-gray-600 font-bold text-blue-400 text-xs">{getTotal(data)}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {Object.keys(sortedAlliances).length === 0 && Object.keys(sortedConstruction).length === 0 && Object.keys(sortedTimeSlots).length === 0 && (
            <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
              <i className="fas fa-chart-bar text-4xl text-gray-500 mb-4"></i>
              <p className="text-xl text-gray-400">No statistics available. Generate a schedule first.</p>
            </div>
          )}
        </div>
      )}

    </div>
  )
}
