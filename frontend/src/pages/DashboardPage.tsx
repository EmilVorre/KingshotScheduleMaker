import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { useAuth } from '../context/AuthContext'
import {
  api,
  type AccountStats,
  type AlliancePlayer,
  type CreateFormRequest,
  type CurrentFormInfo,
  type PredeterminedSlot,
  type Schedule,
} from '../api/client'
import {
  TabProfile,
  TabGiftcodeAutomation,
  TabSwordland,
  TabTriAlliance,
  TabManageServer,
  TabTyrant,
  STANDARD_INTRO_TEXT,
  SUBMISSION_HEADERS,
  SCHEDULE_DAYS,
  TAB_KEYS,
  ALLIANCE_LOCKED_TABS,
  SERVER_ORG_LOCKED_TABS,
  type Tab,
  type ScheduleDayKey,
} from './dashboard'
import {
  sortTimeSlots,
  getSubmissionValue,
  getTotal,
  formatDate,
  copyToClipboard,
  daySlotToTimes,
  type BuildingResearchDaySlot,
} from './dashboard/utils'

interface ExtendedPredeterminedSlot extends PredeterminedSlot {
  lookingUp?: boolean
  lookupError?: string | null
}

export default function DashboardPage() {
  const { accountName } = useParams<{ accountName: string }>()
  const { refresh: refreshAuth, allianceAccess, serverOrgAccess, friendCode } = useAuth()
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const tabParam = searchParams.get('tab') as Tab | null
  const activeTab: Tab = tabParam && TAB_KEYS.includes(tabParam) ? tabParam : 'schedule'

  function selectTab(tab: Tab) {
    setSearchParams({ tab })
  }
  const [sessionValid, setSessionValid] = useState<boolean | null>(null)
  const [serverNumber, setServerNumber] = useState<number | null>(null)
  const [playerId, setPlayerId] = useState<string | null>(null)
  const [inGameName, setInGameName] = useState<string | null>(null)

  // Profile edit state
  const [profileEdit, setProfileEdit] = useState({ account_name: '', server_number: '', in_game_name: '' })
  const [profileSaving, setProfileSaving] = useState(false)
  const [profileError, setProfileError] = useState<string | null>(null)
  const [kingshotIdInput, setKingshotIdInput] = useState('')
  const [kingshotLookingUp, setKingshotLookingUp] = useState(false)
  const [kingshotError, setKingshotError] = useState<string | null>(null)

  // Schedule tab
  const [currentScheduleDay, setCurrentScheduleDay] = useState<ScheduleDayKey>('construction')
  const [scheduleLoading, setScheduleLoading] = useState(false)
  const [scheduleError, setScheduleError] = useState<string | null>(null)
  const [currentSchedule, setCurrentSchedule] = useState<Schedule | null>(null)
  const [editingSlot, setEditingSlot] = useState<{ time: string; player: string } | null>(null)
  const [savingSlot, setSavingSlot] = useState(false)
  const slotInputRef = useRef<HTMLInputElement>(null)

  // Stats tab
  const [statsLoading, setStatsLoading] = useState(false)
  const [statsError, setStatsError] = useState<string | null>(null)
  const [stats, setStats] = useState<AccountStats | null>(null)

  // Create Form tab
  const [config, setConfig] = useState({
    form_name: '',
    kingdom_id: '',
    support_person_name: '',
    alliances: [] as string[],
    include_non_of_above: true,
    construction_truegold_mode: 'truegold_unlocked' as string,
    construction_day_slot: 'monday' as BuildingResearchDaySlot,
    research_day_slot: 'tuesday' as BuildingResearchDaySlot,
  })
  const [creatingForm, setCreatingForm] = useState(false)
  const [configStatus, setConfigStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [createdFormUrl, setCreatedFormUrl] = useState<string | null>(null)
  const [createdFormCode, setCreatedFormCode] = useState<string | null>(null)

  // Current Form tab
  const [currentForm, setCurrentForm] = useState<CurrentFormInfo | null>(null)
  const [loadingCurrentForm, setLoadingCurrentForm] = useState(false)
  const [submissions, setSubmissions] = useState<Record<string, unknown>[] | null>(null)
  const [loadingSubmissions, setLoadingSubmissions] = useState(false)
  const [submissionsError, setSubmissionsError] = useState<string | null>(null)
  const [oldForms, setOldForms] = useState<Array<{ archive_name: string; code: string; name: string; created_at: string; delete_date?: string }>>([])
  const [loadingOldForms, setLoadingOldForms] = useState(false)
  const [reopeningForm, setReopeningForm] = useState(false)
  const [clearScheduleLoading, setClearScheduleLoading] = useState<string | null>(null)
  const [scheduleUrlCopied, setScheduleUrlCopied] = useState(false)

  // CSV Operations tab
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)
  const [uploadStatus, setUploadStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [downloadingCSV, setDownloadingCSV] = useState(false)

  // Generate Schedule tab
  const [predeterminedSlots, setPredeterminedSlots] = useState<ExtendedPredeterminedSlot[]>([])
  const [generatingSchedule, setGeneratingSchedule] = useState(false)
  const [scheduleGenStatus, setScheduleGenStatus] = useState<{
    type: 'success' | 'error'
    message: string
  } | null>(null)

  // Alliance Organisation tab
  const [alliances, setAlliances] = useState<
    Array<{ name: string; slug: string; players: AlliancePlayer[]; owner_account: string; owner_server: number; is_owner: boolean }>
  >([])
  const [loadingAlliances, setLoadingAlliances] = useState(false)
  const [alliancesError, setAlliancesError] = useState<string | null>(null)
  const [addPlayerIdBySlug, setAddPlayerIdBySlug] = useState<Record<string, string>>({})
  const [addingPlayerSlug, setAddingPlayerSlug] = useState<string | null>(null)
  const [addPlayerError, setAddPlayerError] = useState<string | null>(null)
  const addPlayerInputRef = useRef<HTMLInputElement>(null)
  const [removeInputBySlug, setRemoveInputBySlug] = useState<Record<string, string>>({})
  const [removingPlayerSlug, setRemovingPlayerSlug] = useState<string | null>(null)
  const [removePlayerError, setRemovePlayerError] = useState<string | null>(null)
  const [refreshingAllianceSlug, setRefreshingAllianceSlug] = useState<string | null>(null)
  const [refreshNamesError, setRefreshNamesError] = useState<string | null>(null)
  const [inviteFriendCode, setInviteFriendCode] = useState('')
  const [inviteError, setInviteError] = useState<string | null>(null)
  const [inviteSending, setInviteSending] = useState(false)
  const [allianceInvites, setAllianceInvites] = useState<{
    sent: Array<{ id: string; to_friend_code: string; to_account: string; alliance_name: string; status: string }>
    received: Array<{ id: string; from_account: string; alliance_name: string; status: string }>
  }>({ sent: [], received: [] })

  // Alliance Application tab
  const [allianceApp, setAllianceApp] = useState<{
    alliance_tag: string
    alliance_name: string
    contact_player_id: string
    server_number: number
  }>({ alliance_tag: '', alliance_name: '', contact_player_id: '', server_number: 0 })
  const [myApplication, setMyApplication] = useState<{
    id: string
    status: string
    submitted_at: string
    alliance_tag: string
    alliance_name: string
    contact_player_id: string
    server_number: number
  } | null>(null)
  const [loadingMyApplication, setLoadingMyApplication] = useState(false)
  const [submittingApplication, setSubmittingApplication] = useState(false)
  const [applicationError, setApplicationError] = useState<string | null>(null)

  const loadSession = useCallback(async () => {
    const { ok, data } = await api.getSession()
    if (!ok || !data?.account_name || data.account_name !== accountName) {
      setSessionValid(false)
      navigate('/', { replace: true })
    } else {
      setSessionValid(true)
      setServerNumber(data.server_number ?? null)
      setPlayerId(data.player_id ?? null)
      setInGameName(data.in_game_name ?? null)
      setProfileEdit({
        account_name: data.account_name ?? '',
        server_number: String(data.server_number ?? ''),
        in_game_name: data.in_game_name ?? '',
      })
    }
  }, [accountName, navigate])

  useEffect(() => {
    loadSession()
  }, [loadSession])

  // Redirect to alliance-application when user without alliance_access tries to access locked tabs
  useEffect(() => {
    if (
      accountName &&
      !allianceAccess &&
      ALLIANCE_LOCKED_TABS.includes(activeTab)
    ) {
      setSearchParams({ tab: 'alliance-application' })
    }
  }, [accountName, allianceAccess, activeTab, setSearchParams])

  useEffect(() => {
    if (
      accountName &&
      !serverOrgAccess &&
      SERVER_ORG_LOCKED_TABS.includes(activeTab)
    ) {
      setSearchParams({ tab: 'schedule' })
    }
  }, [accountName, serverOrgAccess, activeTab, setSearchParams])

  // Redirect to alliance-organisation when user with alliance_access tries to access alliance-application (tab hidden for them)
  useEffect(() => {
    if (accountName && allianceAccess && activeTab === 'alliance-application') {
      setSearchParams({ tab: 'alliance-organisation' })
    }
  }, [accountName, allianceAccess, activeTab, setSearchParams])

  const loadSchedule = useCallback(
    async (day: ScheduleDayKey) => {
      if (!accountName || !serverNumber) return
      setScheduleLoading(true)
      setScheduleError(null)
      setCurrentSchedule(null)
      setEditingSlot(null)
      const { ok, data, error } = await api.getSchedule(accountName, serverNumber, day)
      setScheduleLoading(false)
      if (ok && data) setCurrentSchedule(data)
      else setScheduleError(error ?? 'Failed to load schedule')
    },
    [accountName, serverNumber]
  )

  const loadStats = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setStatsLoading(true)
    setStatsError(null)
    const { ok, data, error } = await api.getStats(accountName, serverNumber)
    setStatsLoading(false)
    if (ok && data) setStats(data)
    else setStatsError(error ?? 'Failed to load statistics')
  }, [accountName, serverNumber])

  const loadCurrentForm = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoadingCurrentForm(true)
    const { ok, data } = await api.getCurrentForm(accountName, serverNumber)
    setLoadingCurrentForm(false)
    const form = (data as { form?: CurrentFormInfo })?.form ?? data
    if (ok && form) {
      setCurrentForm(form as CurrentFormInfo)
      const configData = (data as { form?: { config?: { predetermined_slots?: PredeterminedSlot[] } } })?.form
        ?.config
      if (configData?.predetermined_slots) {
        setPredeterminedSlots(
          configData.predetermined_slots.map((s) => ({
            ...s,
            lookingUp: false,
            lookupError: null,
          }))
        )
      } else {
        setPredeterminedSlots([])
      }
    } else {
      setCurrentForm(null)
      setPredeterminedSlots([])
      setSubmissions(null)
    }
  }, [accountName, serverNumber])

  const loadSubmissions = useCallback(async () => {
    if (!accountName || !serverNumber || !currentForm) return
    setLoadingSubmissions(true)
    setSubmissionsError(null)
    const { ok, data, error } = await api.getFormSubmissions(accountName, serverNumber)
    setLoadingSubmissions(false)
    const subs = (data as { submissions?: Record<string, unknown>[] })?.submissions ?? []
    if (ok) setSubmissions(subs)
    else {
      setSubmissionsError(error ?? 'Failed to load submissions')
      setSubmissions([])
    }
  }, [accountName, serverNumber, currentForm])

  const loadConfig = useCallback(async () => {
    if (!accountName || !serverNumber) return
    const { ok, data } = await api.getPreviousFormConfig(accountName, serverNumber)
    if (ok && data?.config) {
      const c = data.config
      setConfig({
        form_name: '',
        kingdom_id: (c as { kingdom_id?: string }).kingdom_id ?? '',
        support_person_name: (c as { support_person_name?: string }).support_person_name ?? '',
        alliances: ((c.alliances ?? []) as string[]).filter((a) => a !== 'Non of the above'),
        include_non_of_above: (c as { include_non_of_above?: boolean }).include_non_of_above !== false,
        construction_truegold_mode:
          (c as { construction_truegold_mode?: string }).construction_truegold_mode ?? 'truegold_unlocked',
        construction_day_slot:
          (c as { construction_day_slot?: BuildingResearchDaySlot }).construction_day_slot ?? 'monday',
        research_day_slot:
          (c as { research_day_slot?: BuildingResearchDaySlot }).research_day_slot ?? 'tuesday',
      })
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    if (sessionValid && accountName && serverNumber) {
      loadStats()
      loadSchedule('construction')
      loadConfig()
      loadCurrentForm()
    }
  }, [sessionValid, accountName, serverNumber])

  useEffect(() => {
    if (activeTab === 'current-form' && currentForm && !submissions && !loadingSubmissions) {
      loadSubmissions()
    }
  }, [activeTab, currentForm, submissions, loadingSubmissions, loadSubmissions])

  const loadOldForms = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoadingOldForms(true)
    const { ok, data } = await api.listOldForms(accountName, serverNumber)
    setLoadingOldForms(false)
    if (ok && (data as { old_forms?: unknown[] })?.old_forms) {
      setOldForms((data as { old_forms: Array<{ archive_name: string; code: string; name: string; created_at: string; delete_date?: string }> }).old_forms)
    } else {
      setOldForms([])
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    if (activeTab === 'current-form') {
      loadOldForms()
    }
  }, [activeTab, loadOldForms])

  const loadAlliances = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoadingAlliances(true)
    setAlliancesError(null)
    const { ok, data, error } = await api.listAlliances(accountName, serverNumber)
    setLoadingAlliances(false)
    if (ok && data?.alliances) setAlliances(data.alliances)
    else setAlliancesError(error ?? 'Failed to load alliances')
  }, [accountName, serverNumber])

  const loadAllianceInvites = useCallback(async () => {
    if (!accountName || !serverNumber) return
    const { ok, data } = await api.listAllianceInvites(accountName, serverNumber)
    if (ok && data) setAllianceInvites({ sent: data.sent ?? [], received: data.received ?? [] })
  }, [accountName, serverNumber])

  const loadFriendCode = useCallback(async () => {
    if (!accountName || !serverNumber) return
    await api.getFriendCode(accountName, serverNumber)
    refreshAuth()
  }, [accountName, serverNumber, refreshAuth])

  useEffect(() => {
    if (activeTab === 'alliance-organisation') {
      loadAlliances()
      loadAllianceInvites()
      loadFriendCode()
    }
  }, [activeTab, loadAlliances, loadAllianceInvites, loadFriendCode])

  const loadMyAllianceApplication = useCallback(async () => {
    setLoadingMyApplication(true)
    setApplicationError(null)
    const { ok, data } = await api.getMyAllianceApplication()
    setLoadingMyApplication(false)
    if (ok && data?.application) {
      setMyApplication(data.application)
    } else {
      setMyApplication(null)
    }
  }, [])

  useEffect(() => {
    if (activeTab === 'alliance-application') {
      loadMyAllianceApplication()
    }
  }, [activeTab, loadMyAllianceApplication])

  async function handleReopenForm(archiveName: string) {
    if (!accountName || !serverNumber) return
    setReopeningForm(true)
    const { ok, error } = await api.reopenForm(accountName, serverNumber, archiveName)
    setReopeningForm(false)
    if (ok) {
      loadCurrentForm()
      loadOldForms()
    } else {
      setConfigStatus({ type: 'error', message: error ?? 'Failed to reopen form' })
    }
  }

  async function handleClearSchedule(day?: 'construction' | 'research' | 'troops') {
    if (!accountName || !serverNumber) return
    const key = day ?? 'all'
    setClearScheduleLoading(key)
    const { ok, error } = await api.clearSchedule(accountName, serverNumber, day)
    setClearScheduleLoading(null)
    if (ok) {
      loadSchedule(currentScheduleDay)
    } else {
      setScheduleError(error ?? 'Failed to clear schedule')
    }
  }

  function selectScheduleDay(day: ScheduleDayKey) {
    setCurrentScheduleDay(day)
    loadSchedule(day)
  }

  function startEditSlot(slot: { time: string; player?: string }) {
    setEditingSlot({ time: slot.time, player: slot.player ?? '' })
    setTimeout(() => slotInputRef.current?.focus(), 0)
  }

  function cancelEdit() {
    setEditingSlot(null)
  }

  async function saveSlot(slot: { time: string }) {
    if (!accountName || !serverNumber || savingSlot || !editingSlot) return
    setSavingSlot(true)
    const playerValue = editingSlot.player?.trim() ?? ''
    const { ok, error } = await api.updateScheduleSlot(
      accountName,
      serverNumber,
      currentScheduleDay,
      slot.time,
      playerValue
    )
    setSavingSlot(false)
    if (ok) {
      setEditingSlot(null)
      loadSchedule(currentScheduleDay)
    } else {
      alert('Error: ' + (error ?? 'Failed to update slot'))
    }
  }

  const sortedAlliances = stats?.alliance_counts
    ? Object.entries(stats.alliance_counts).sort((a, b) => getTotal(b[1]) - getTotal(a[1]))
    : []
  const sortedConstructionTimeSlots = stats?.construction_time_slot_popularity
    ? sortTimeSlots(stats.construction_time_slot_popularity, stats.construction_start_time ?? '00:00')
    : {}
  const sortedResearchTimeSlots = stats?.research_time_slot_popularity
    ? sortTimeSlots(stats.research_time_slot_popularity, stats.research_start_time ?? '00:00')
    : {}
  const sortedTroopsTimeSlots = stats?.troops_time_slot_popularity
    ? sortTimeSlots(stats.troops_time_slot_popularity, stats.troops_start_time ?? '00:00')
    : {}
  const sortedTimeSlots = stats?.time_slot_popularity
    ? Object.entries(stats.time_slot_popularity).sort((a, b) => a[0].localeCompare(b[0]))
    : []

  async function handleCreateForm(e?: React.FormEvent) {
    e?.preventDefault()
    if (!accountName || !serverNumber) return
    const alliances = config.alliances.filter((a) => a.trim() !== '')
    if (alliances.length === 0) {
      setConfigStatus({ type: 'error', message: 'Error: At least one alliance must be specified' })
      return
    }
    if (!config.kingdom_id?.trim()) {
      setConfigStatus({ type: 'error', message: 'Error: Kingdom ID is required' })
      return
    }
    if (config.construction_day_slot === config.research_day_slot) {
      setConfigStatus({
        type: 'error',
        message: 'Error: Construction and Research must be on different days. Please choose a different day for one of them.',
      })
      return
    }
    setCreatingForm(true)
    setConfigStatus(null)
    setCreatedFormUrl(null)
    setCreatedFormCode(null)
    const request: CreateFormRequest = {
      name: config.form_name?.trim() || undefined,
      kingdom_id: config.kingdom_id.trim(),
      alliances,
      include_non_of_above: config.include_non_of_above,
      construction_truegold_mode: config.construction_truegold_mode,
      construction_times: daySlotToTimes(config.construction_day_slot),
      research_times: daySlotToTimes(config.research_day_slot),
      troops_times: { start_time: '00:00', end_time: undefined },
      construction_day_slot: config.construction_day_slot,
      research_day_slot: config.research_day_slot,
      intro_text: STANDARD_INTRO_TEXT,
      support_person_name: config.support_person_name?.trim() || undefined,
    }
    const { ok, data, error } = await api.createForm(accountName, serverNumber, request)
    setCreatingForm(false)
    if (ok && data) {
      const url = (data as { url?: string }).url ?? (data as { form_url?: string }).form_url
      const code = (data as { code?: string }).code ?? (data as { form_code?: string }).form_code
      setCreatedFormUrl(url ?? null)
      setCreatedFormCode(code ?? null)
      setConfigStatus({ type: 'success', message: 'Form created successfully!' })
      await loadCurrentForm()
      selectTab('current-form')
    } else {
      setConfigStatus({ type: 'error', message: 'Error: ' + (error ?? 'Failed to create form') })
    }
  }

  async function handleDownloadCSV() {
    if (!accountName || !serverNumber) return
    setDownloadingCSV(true)
    try {
      const res = await api.downloadFormCsv(accountName, serverNumber)
      if (res.ok) {
        const blob = await res.blob()
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `form_submissions_${accountName}_${serverNumber}.csv`
        a.click()
        URL.revokeObjectURL(url)
      }
    } finally {
      setDownloadingCSV(false)
    }
  }

  async function handleUpload(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || !serverNumber || !selectedFile) {
      setUploadStatus({ type: 'error', message: 'Please select a file' })
      return
    }
    setUploading(true)
    setUploadStatus(null)
    const res = await api.uploadCsv(accountName, serverNumber, selectedFile)
    setUploading(false)
    if (res.ok && (res.data as { success?: boolean })?.success) {
      setUploadStatus({
        type: 'success',
        message: (res.data as { message?: string })?.message ?? 'Schedule generated successfully!',
      })
      setSelectedFile(null)
      loadSchedule(currentScheduleDay)
      loadStats()
    } else {
      setUploadStatus({ type: 'error', message: 'Error: ' + (res.error ?? 'Upload failed') })
    }
  }

  async function lookupPlayer(index: number) {
    const slot = predeterminedSlots[index]
    if (!slot.player_id?.trim()) {
      setPredeterminedSlots((prev) => {
        const next = [...prev]
        next[index] = { ...next[index], alliance: '', name: '', lookupError: null }
        return next
      })
      return
    }
    setPredeterminedSlots((prev) => {
      const next = [...prev]
      next[index] = { ...next[index], lookingUp: true, lookupError: null }
      return next
    })
    const { ok, data } = await api.getPlayerById(accountName!, serverNumber!, slot.player_id!.trim())
    const player = (data as { player?: { name?: string; alliance?: string } })?.player
    setPredeterminedSlots((prev) => {
      const next = [...prev]
      next[index] = {
        ...next[index],
        lookingUp: false,
        alliance: player?.alliance ?? '',
        name: player?.name ?? '',
        lookupError: ok && player ? null : (data as { error?: string })?.error ?? 'Player not found',
      }
      return next
    })
  }

  async function savePredeterminedSlots(): Promise<boolean> {
    if (!currentForm || !accountName || !serverNumber) return false
    const slots = predeterminedSlots.map((s) => ({
      day: s.day,
      time: s.time,
      player_id: s.player_id || undefined,
      alliance: s.alliance,
      name: s.name,
    }))
    const { ok } = await api.updateFormConfig(accountName, serverNumber, slots)
    if (ok) await loadCurrentForm()
    return !!ok
  }

  async function handleGenerateSchedule(append: boolean, day?: 'construction' | 'research' | 'troops') {
    if (!accountName || !serverNumber) return
    setGeneratingSchedule(true)
    setScheduleGenStatus(null)
    if (predeterminedSlots.length > 0 && !day) {
      const saved = await savePredeterminedSlots()
      if (!saved) {
        setScheduleGenStatus({
          type: 'error',
          message: 'Failed to save predetermined slots. Please try again.',
        })
        setGeneratingSchedule(false)
        return
      }
    }
    const { ok, data, error } = await api.generateSchedule(append, day)
    setGeneratingSchedule(false)
    if (ok && (data as { success?: boolean })?.success) {
      setScheduleGenStatus({
        type: 'success',
        message: (data as { message?: string })?.message ?? 'Schedule generated successfully!',
      })
      loadSchedule(currentScheduleDay)
      loadStats()
    } else {
      setScheduleGenStatus({
        type: 'error',
        message: (data as { error?: string })?.error ?? error ?? 'Failed to generate schedule',
      })
    }
  }

  if (sessionValid === null) {
    return (
      <div className="container mx-auto px-4 py-8 flex justify-center items-center min-h-[50vh]">
        <i className="fas fa-spinner fa-spin text-4xl text-blue-400"></i>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-7xl">
      <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
        <div className="min-h-[200px]">
          {/* Profile Tab */}
          {activeTab === 'profile' && (
            <TabProfile
              accountName={accountName ?? null}
              serverNumber={serverNumber}
              playerId={playerId}
              inGameName={inGameName}
              friendCode={friendCode ?? null}
              profileEdit={profileEdit}
              profileSaving={profileSaving}
              profileError={profileError}
              kingshotIdInput={kingshotIdInput}
              kingshotLookingUp={kingshotLookingUp}
              kingshotError={kingshotError}
              setProfileEdit={setProfileEdit}
              setProfileSaving={setProfileSaving}
              setProfileError={setProfileError}
              setKingshotIdInput={setKingshotIdInput}
              setKingshotLookingUp={setKingshotLookingUp}
              setKingshotError={setKingshotError}
              setServerNumber={setServerNumber}
              setInGameName={setInGameName}
              setPlayerId={setPlayerId}
              refreshAuth={refreshAuth}
              navigate={navigate}
            />
          )}

          {/* Schedule Tab */}
          {activeTab === 'schedule' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-orange-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-calendar-check text-orange-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Schedule</h2>
                <p className="text-gray-400">View appointment schedules for each day</p>
              </div>

              <div className="bg-gray-800 rounded-xl shadow-xl p-6 mb-6 border border-gray-700">
                <p className="text-xs font-medium text-gray-500 uppercase tracking-wider mb-3">View schedule</p>
                {accountName && currentForm?.code && (
                  <div className="flex flex-wrap items-center gap-2 mb-4 p-3 bg-gray-900/50 rounded-lg">
                    <span className="text-sm text-gray-400">Public schedule:</span>
                    <code className="text-blue-400 font-mono text-sm flex-1 min-w-0 truncate">
                      /{accountName}/{currentForm.code}
                    </code>
                    <button
                      type="button"
                      onClick={() => {
                        copyToClipboard(`/${accountName}/${currentForm.code}`)
                        setScheduleUrlCopied(true)
                        setTimeout(() => setScheduleUrlCopied(false), 2000)
                      }}
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-colors flex items-center gap-2 shrink-0"
                    >
                      <i className={`fas ${scheduleUrlCopied ? 'fa-check' : 'fa-copy'}`}></i>
                      {scheduleUrlCopied ? 'Copied!' : 'Copy URL'}
                    </button>
                  </div>
                )}
                <div className="flex flex-col sm:flex-row gap-2 mb-4">
                  {(Object.keys(SCHEDULE_DAYS) as ScheduleDayKey[]).map((key) => (
                    <button
                      key={key}
                      onClick={() => selectScheduleDay(key)}
                      className={`flex items-center justify-center gap-2 px-5 py-3 rounded-lg font-semibold transition-all ${
                        SCHEDULE_DAYS[key].buttonClass
                      } ${currentScheduleDay === key ? SCHEDULE_DAYS[key].ringClass : 'opacity-90 hover:opacity-100'}`}
                    >
                      <i className={`${SCHEDULE_DAYS[key].icon}`}></i>
                      {SCHEDULE_DAYS[key].name}
                    </button>
                  ))}
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    onClick={() => handleClearSchedule()}
                    disabled={!!clearScheduleLoading}
                    className="px-4 py-2 bg-red-600/90 hover:bg-red-600 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
                  >
                    {clearScheduleLoading === 'all' ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-trash mr-2"></i>}
                    Clear All
                  </button>
                  <button
                    onClick={() => handleClearSchedule(currentScheduleDay)}
                    disabled={!!clearScheduleLoading}
                    className="px-3 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 rounded-lg text-sm font-medium transition-colors flex items-center gap-1.5"
                  >
                    {clearScheduleLoading === currentScheduleDay ? (
                      <i className="fas fa-spinner fa-spin"></i>
                    ) : (
                      <i className={`${SCHEDULE_DAYS[currentScheduleDay].icon} text-gray-400`}></i>
                    )}
                    Clear {SCHEDULE_DAYS[currentScheduleDay].name}
                  </button>
                </div>
              </div>

              {scheduleLoading && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
                  <p className="text-xl text-gray-400">Loading schedule...</p>
                </div>
              )}

              {!scheduleLoading && scheduleError && (
                <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {scheduleError}
                </div>
              )}

              {!scheduleLoading && !scheduleError && currentSchedule && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                  <h2 className="text-3xl font-bold text-white mb-6 text-center">
                    {currentSchedule.day_name}
                  </h2>
                  <p className="text-sm text-gray-400 mb-4 text-center">
                    Click on any slot to edit the player name
                  </p>
                  <div className="border-2 border-gray-700 rounded-lg overflow-hidden">
                    {currentSchedule.appointments?.map((slot) => (
                      <div
                        key={slot.time}
                        onClick={() => startEditSlot(slot)}
                        className={`flex items-center p-3 border-b border-gray-700 hover:bg-gray-700/50 transition-colors cursor-pointer ${
                          slot.is_empty ? 'opacity-60' : ''
                        } ${editingSlot?.time === slot.time ? 'bg-blue-900/30' : ''}`}
                      >
                        <span
                          className={
                            slot.is_empty
                              ? 'w-24 font-bold text-gray-500'
                              : 'w-24 font-bold text-blue-400'
                          }
                        >
                          {slot.time}
                        </span>
                        {editingSlot?.time === slot.time ? (
                          <div className="flex-1 flex items-center gap-2">
                            <input
                              ref={slotInputRef}
                              type="text"
                              value={editingSlot.player}
                              onChange={(e) =>
                                setEditingSlot((p) => (p ? { ...p, player: e.target.value } : null))
                              }
                              onKeyDown={(e) => {
                                if (e.key === 'Enter') saveSlot(slot)
                                if (e.key === 'Escape') cancelEdit()
                              }}
                              className="flex-1 px-3 py-1 bg-gray-700 border border-blue-500 rounded text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                              placeholder="[alliance] name or empty to clear"
                              onClick={(e) => e.stopPropagation()}
                            />
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                saveSlot(slot)
                              }}
                              className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white rounded transition-colors"
                            >
                              <i className="fas fa-check"></i>
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                cancelEdit()
                              }}
                              className="px-3 py-1 bg-red-600 hover:bg-red-700 text-white rounded transition-colors"
                            >
                              <i className="fas fa-times"></i>
                            </button>
                          </div>
                        ) : (
                          <div className="flex-1">
                            {slot.is_empty ? (
                              <span className="text-gray-500 italic">[EMPTY]</span>
                            ) : (
                              <span className="text-gray-200 font-medium">{slot.player}</span>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {!scheduleLoading && !scheduleError && !currentSchedule && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-calendar-times text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400">
                    No schedule data available. Generate a schedule first.
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Alliance Application Tab */}
          {activeTab === 'alliance-application' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-indigo-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-file-signature text-indigo-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Alliance Application</h2>
                <p className="text-gray-400">
                  Apply for access to alliance organisation tools on this site.
                </p>
              </div>

              <div className="bg-amber-900/30 border border-amber-600/50 rounded-lg p-4 mb-8">
                <p className="text-amber-200 text-sm">
                  <i className="fas fa-info-circle mr-2"></i>
                  This site is currently in development. To ensure stable performance and stay within our resource and external API limits, we ask alliances to apply before using these features. Approved applications will receive full access to the Alliance Organisation tools.
                </p>
              </div>

              {loadingMyApplication && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-indigo-400 mb-4"></i>
                  <p className="text-gray-400">Loading application status...</p>
                </div>
              )}

              {!loadingMyApplication && myApplication && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700 mb-8">
                  <h3 className="text-lg font-bold text-white mb-4 flex items-center gap-2">
                    <i className="fas fa-clipboard-check text-indigo-400"></i>
                    Your Application Status
                  </h3>
                  <div className="flex flex-wrap gap-4 items-center">
                    <span
                      className={`px-3 py-1 rounded-full text-sm font-medium ${
                        myApplication.status === 'approved'
                          ? 'bg-green-900/50 text-green-300'
                          : myApplication.status === 'rejected'
                            ? 'bg-red-900/50 text-red-300'
                            : 'bg-amber-900/50 text-amber-300'
                      }`}
                    >
                      {myApplication.status.charAt(0).toUpperCase() + myApplication.status.slice(1)}
                    </span>
                    <span className="text-gray-400 text-sm">
                      Submitted: {new Date(myApplication.submitted_at).toLocaleString()}
                    </span>
                  </div>
                  <div className="mt-4 text-gray-300 text-sm space-y-1">
                    <p><strong>Alliance:</strong> [{myApplication.alliance_tag}] {myApplication.alliance_name}</p>
                    <p><strong>Contact:</strong> {myApplication.contact_player_id} · Server {myApplication.server_number}</p>
                  </div>
                  {myApplication.status === 'approved' && (
                    <p className="mt-4 text-green-400 text-sm">
                      <i className="fas fa-check-circle mr-1"></i>
                      You have full access to Alliance Organisation tools.
                    </p>
                  )}
                  {myApplication.status === 'rejected' && (
                    <p className="mt-4 text-amber-400 text-sm">
                      Your application was not approved. You may contact support if you have questions.
                    </p>
                  )}
                </div>
              )}

              {!loadingMyApplication && (!myApplication || myApplication.status === 'rejected') && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700">
                  <h3 className="text-xl font-bold text-white mb-4 flex items-center">
                    <i className="fas fa-edit text-indigo-400 mr-2"></i>
                    Submit Application
                  </h3>
                  <div className="space-y-4 max-w-xl">
                    <div>
                      <label className="block text-sm font-medium text-gray-300 mb-1">Alliance Tag</label>
                      <input
                        type="text"
                        value={allianceApp.alliance_tag}
                        onChange={(e) => {
                          setAllianceApp((a) => ({ ...a, alliance_tag: e.target.value }))
                          setApplicationError(null)
                        }}
                        placeholder="e.g. COB"
                        maxLength={16}
                        className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-indigo-500 outline-none"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-300 mb-1">Alliance Name</label>
                      <input
                        type="text"
                        value={allianceApp.alliance_name}
                        onChange={(e) => {
                          setAllianceApp((a) => ({ ...a, alliance_name: e.target.value }))
                          setApplicationError(null)
                        }}
                        placeholder="e.g. Slaughterhouse"
                        className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-indigo-500 outline-none"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-300 mb-1">Contact Person In-Game ID</label>
                      <input
                        type="text"
                        value={allianceApp.contact_player_id}
                        onChange={(e) => {
                          setAllianceApp((a) => ({
                            ...a,
                            contact_player_id: e.target.value.replace(/\D/g, ''),
                          }))
                          setApplicationError(null)
                        }}
                        placeholder="Numeric player ID"
                        className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-indigo-500 outline-none"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-300 mb-1">Server Number</label>
                      <input
                        type="number"
                        min={1}
                        value={allianceApp.server_number || serverNumber || ''}
                        onChange={(e) => {
                          const v = parseInt(e.target.value, 10)
                          setAllianceApp((a) => ({ ...a, server_number: isNaN(v) ? 0 : v }))
                          setApplicationError(null)
                        }}
                        placeholder="e.g. 140"
                        className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-indigo-500 outline-none"
                      />
                    </div>
                    <button
                      onClick={async () => {
                        setSubmittingApplication(true)
                        setApplicationError(null)
                        const { ok, error } = await api.submitAllianceApplication({
                          alliance_tag: allianceApp.alliance_tag.trim(),
                          alliance_name: allianceApp.alliance_name.trim(),
                          contact_player_id: allianceApp.contact_player_id.trim(),
                          server_number: allianceApp.server_number || serverNumber || 1,
                        })
                        setSubmittingApplication(false)
                        if (ok) {
                          await refreshAuth()
                          loadMyAllianceApplication()
                        } else {
                          setApplicationError(error ?? 'Failed to submit application')
                        }
                      }}
                      disabled={
                        submittingApplication ||
                        !allianceApp.alliance_tag.trim() ||
                        !allianceApp.alliance_name.trim() ||
                        !allianceApp.contact_player_id.trim()
                      }
                      className="px-6 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-all"
                    >
                      {submittingApplication ? (
                        <>
                          <i className="fas fa-spinner fa-spin mr-2"></i>Submitting...
                        </>
                      ) : (
                        <>
                          <i className="fas fa-paper-plane mr-2"></i>Submit Application
                        </>
                      )}
                    </button>
                    {applicationError && (
                      <p className="text-red-400 text-sm">
                        <i className="fas fa-exclamation-circle mr-1"></i>
                        {applicationError}
                      </p>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Alliance Organisation Tab */}
          {activeTab === 'alliance-organisation' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-indigo-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-sitemap text-indigo-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Alliance Organisation</h2>
                <p className="text-gray-400">
                  Add and remove players from alliance lists. Enter a player ID to add; use ID or name to remove.
                </p>
              </div>

              {/* Friend code & invite box */}
              <div className="mb-6 p-4 bg-gray-800/80 rounded-lg border border-gray-700">
                <div className="flex flex-wrap gap-4 items-start">
                  <div className="flex-1 min-w-[200px]">
                    <label className="block text-sm font-medium text-gray-300 mb-1">Your friend code</label>
                    <div className="flex items-center gap-2">
                      <code className="px-3 py-2 bg-gray-700 rounded font-mono text-indigo-300">
                        {friendCode || 'Loading...'}
                      </code>
                      <button
                        type="button"
                        onClick={() => friendCode && navigator.clipboard.writeText(friendCode)}
                        className="text-gray-400 hover:text-white"
                        title="Copy"
                      >
                        <i className="fas fa-copy"></i>
                      </button>
                    </div>
                    <p className="text-xs text-gray-500 mt-1">Share this so others can invite you to edit their alliance</p>
                  </div>
                  <div className="flex-1 min-w-[200px]">
                    <label className="block text-sm font-medium text-gray-300 mb-1">
                      Invite admins by friend code {alliances.some((a) => a.is_owner) ? '(only alliance owner)' : '(need your own alliance)'}
                    </label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        value={inviteFriendCode}
                        onChange={(e) => {
                          setInviteFriendCode(e.target.value.replace(/\s/g, '').slice(0, 12))
                          setInviteError(null)
                        }}
                        placeholder="12-character code"
                        maxLength={12}
                        disabled={!alliances.some((a) => a.is_owner)}
                        className="flex-1 px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white font-mono placeholder-gray-500 disabled:opacity-50"
                      />
                      <button
                        onClick={async () => {
                          if (!accountName || !serverNumber || !inviteFriendCode.trim()) return
                          setInviteSending(true)
                          setInviteError(null)
                          const { ok, error } = await api.createAllianceInvite(
                            accountName,
                            serverNumber,
                            inviteFriendCode.trim()
                          )
                          setInviteSending(false)
                          if (ok) {
                            setInviteFriendCode('')
                            loadAllianceInvites()
                          } else {
                            setInviteError(error ?? 'Failed to send invite')
                          }
                        }}
                        disabled={inviteSending || inviteFriendCode.trim().length !== 12 || !alliances.some((a) => a.is_owner)}
                        className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded font-medium"
                      >
                        {inviteSending ? <i className="fas fa-spinner fa-spin"></i> : 'Invite'}
                      </button>
                    </div>
                    {inviteError && <p className="text-red-400 text-sm mt-1">{inviteError}</p>}
                  </div>
                </div>
                {allianceInvites.received.length > 0 && (
                  <div className="mt-4 pt-4 border-t border-gray-700">
                    <p className="text-sm font-medium text-gray-300 mb-2">Pending invites (accept to gain edit access)</p>
                    <div className="space-y-2">
                      {allianceInvites.received.map((inv) => (
                        <div
                          key={inv.id}
                          className="flex items-center justify-between py-2 px-3 bg-gray-700/50 rounded"
                        >
                          <span className="text-gray-300">
                            {inv.from_account} invited you to <strong>{inv.alliance_name}</strong>
                          </span>
                          <div className="flex gap-2">
                            <button
                              onClick={async () => {
                                if (!accountName || !serverNumber) return
                                const { ok } = await api.acceptAllianceInvite(accountName, serverNumber, inv.id)
                                if (ok) {
                                  loadAllianceInvites()
                                  loadAlliances()
                                  refreshAuth()
                                }
                              }}
                              className="px-3 py-1 bg-green-600 hover:bg-green-500 text-white rounded text-sm"
                            >
                              Accept
                            </button>
                            <button
                              onClick={async () => {
                                if (!accountName || !serverNumber) return
                                const { ok } = await api.rejectAllianceInvite(accountName, serverNumber, inv.id)
                                if (ok) loadAllianceInvites()
                              }}
                              className="px-3 py-1 bg-gray-600 hover:bg-gray-500 text-white rounded text-sm"
                            >
                              Reject
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                {alliances.some((a) => a.is_owner) && allianceInvites.sent.filter((i) => i.status === 'accepted').length > 0 && (
                  <div className="mt-4 pt-4 border-t border-gray-700">
                    <p className="text-sm font-medium text-gray-300 mb-2">
                      Alliance admins <span className="text-gray-500 font-normal">(only you can invite or remove)</span>
                    </p>
                    <div className="space-y-2">
                      {allianceInvites.sent
                        .filter((i) => i.status === 'accepted')
                        .map((inv) => (
                          <div
                            key={inv.id}
                            className="flex items-center justify-between py-2 px-3 bg-gray-700/50 rounded"
                          >
                            <span className="text-gray-300">
                              {inv.to_account || inv.to_friend_code}
                            </span>
                            <button
                              onClick={async () => {
                                if (!accountName || !serverNumber) return
                                const { ok } = await api.revokeAllianceInvite(accountName, serverNumber, inv.id)
                                if (ok) {
                                  loadAllianceInvites()
                                  loadAlliances()
                                }
                              }}
                              className="px-3 py-1 bg-red-600 hover:bg-red-500 text-white rounded text-sm"
                            >
                              Remove
                            </button>
                          </div>
                        ))}
                    </div>
                  </div>
                )}
              </div>

              {addPlayerError && (
                <div className="mb-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {addPlayerError}
                </div>
              )}
              {removePlayerError && (
                <div className="mb-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {removePlayerError}
                </div>
              )}
              {refreshNamesError && (
                <div className="mb-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {refreshNamesError}
                </div>
              )}

              {loadingAlliances && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-indigo-400 mb-4"></i>
                  <p className="text-gray-400">Loading alliances...</p>
                </div>
              )}

              {!loadingAlliances && alliancesError && (
                <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {alliancesError}
                </div>
              )}

              {!loadingAlliances && !alliancesError && alliances.length === 0 && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-users text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400 mb-2">No alliance assigned</p>
                  <p className="text-gray-500 text-sm">
                    Your alliance information is loaded from your approved application. If you believe this is an error, please contact support.
                  </p>
                </div>
              )}

              {!loadingAlliances && !alliancesError && alliances.length > 0 && (
                <div className="space-y-6">
                  {alliances.map((alliance) => {
                    const allianceKey = `${alliance.owner_account}:${alliance.owner_server}:${alliance.slug}`
                    return (
                    <div
                      key={allianceKey}
                      className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700"
                    >
                      <h4 className="text-lg font-bold text-white mb-4 flex items-center gap-2">
                        <i className="fas fa-flag text-indigo-400"></i>
                        {alliance.name}
                        {!alliance.is_owner && (
                          <span className="text-xs bg-gray-600 text-gray-300 px-2 py-0.5 rounded">Shared with you</span>
                        )}
                        <span className="text-indigo-300 font-normal text-sm">
                          ({alliance.players.length} player{alliance.players.length !== 1 ? 's' : ''})
                        </span>
                        <button
                          onClick={async () => {
                            setRefreshingAllianceSlug(allianceKey)
                            setRefreshNamesError(null)
                            const { ok, error } = await api.refreshAllianceNames(
                              alliance.owner_account,
                              alliance.owner_server,
                              alliance.slug
                            )
                            setRefreshingAllianceSlug(null)
                            if (ok) {
                              loadAlliances()
                            } else {
                              setRefreshNamesError(error ?? 'Failed to refresh names')
                            }
                          }}
                          disabled={refreshingAllianceSlug !== null || alliance.players.length === 0}
                          className="ml-auto px-3 py-1.5 text-sm bg-amber-600 hover:bg-amber-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium"
                          title="Refetch names, castle levels, and avatars from the game API"
                        >
                          {refreshingAllianceSlug === allianceKey ? (
                            <>
                              <i className="fas fa-spinner fa-spin mr-2"></i>Refreshing...
                            </>
                          ) : (
                            <>
                              <i className="fas fa-sync-alt mr-2"></i>Refresh names
                            </>
                          )}
                        </button>
                      </h4>
                      {alliance.players.length > 0 && (
                        <p className="text-amber-200/80 text-xs mb-3 -mt-2">
                          <i className="fas fa-info-circle mr-1"></i>
                          Refreshing names fetches updated data for each player from the game. This may take a few minutes for large alliances.
                        </p>
                      )}

                      {/* Add player - Player ID only */}
                      <div className="flex flex-wrap gap-2 items-end mb-4">
                        <div className="flex-1 min-w-[160px]">
                          <label className="block text-sm font-medium text-gray-300 mb-1">Add player (ID)</label>
                          <input
                            ref={addPlayerInputRef}
                            type="text"
                            value={addPlayerIdBySlug[allianceKey] ?? ''}
                            onChange={(e) => {
                              setAddPlayerIdBySlug((prev) => ({
                                ...prev,
                                [allianceKey]: e.target.value.replace(/\D/g, ''),
                              }))
                              setAddPlayerError(null)
                            }}
                            onKeyDown={async (e) => {
                              if (e.key !== 'Enter') return
                              const playerId = (addPlayerIdBySlug[allianceKey] ?? '').trim()
                              if (!playerId) return
                              e.preventDefault()
                              setAddingPlayerSlug(allianceKey)
                              setAddPlayerError(null)
                              const { ok, error } = await api.addAllianceMember(
                                alliance.owner_account,
                                alliance.owner_server,
                                alliance.name,
                                playerId
                              )
                              setAddingPlayerSlug(null)
                              if (ok) {
                                setAddPlayerIdBySlug((prev) => ({ ...prev, [allianceKey]: '' }))
                                loadAlliances()
                                addPlayerInputRef.current?.focus()
                              } else {
                                setAddPlayerError(error ?? 'Failed to add player')
                              }
                            }}
                            placeholder="Player ID"
                            className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-indigo-500 outline-none"
                          />
                        </div>
                        <button
                          onClick={async () => {
                            const playerId = (addPlayerIdBySlug[allianceKey] ?? '').trim()
                            if (!playerId) return
                            setAddingPlayerSlug(allianceKey)
                            setAddPlayerError(null)
                            const { ok, error } = await api.addAllianceMember(
                              alliance.owner_account,
                              alliance.owner_server,
                              alliance.name,
                              playerId
                            )
                            setAddingPlayerSlug(null)
                            if (ok) {
                              setAddPlayerIdBySlug((prev) => ({ ...prev, [allianceKey]: '' }))
                              loadAlliances()
                              addPlayerInputRef.current?.focus()
                            } else {
                              setAddPlayerError(error ?? 'Failed to add player')
                            }
                          }}
                          disabled={addingPlayerSlug !== null || !(addPlayerIdBySlug[allianceKey] ?? '').trim()}
                          className="w-[130px] shrink-0 px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-all"
                        >
                          {addingPlayerSlug === allianceKey ? (
                            <i className="fas fa-spinner fa-spin mr-2"></i>
                          ) : (
                            <i className="fas fa-plus mr-2"></i>
                          )}
                          Add
                        </button>
                      </div>

                      {/* Remove player - ID or name */}
                      <div className="flex flex-wrap gap-2 items-end mb-4">
                        <div className="flex-1 min-w-[160px]">
                          <label className="block text-sm font-medium text-gray-300 mb-1">Remove player (ID or name)</label>
                          <input
                            type="text"
                            value={removeInputBySlug[allianceKey] ?? ''}
                            onChange={(e) => {
                              setRemoveInputBySlug((prev) => ({ ...prev, [allianceKey]: e.target.value }))
                              setRemovePlayerError(null)
                            }}
                            placeholder="Player ID or name"
                            className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-indigo-500 outline-none"
                          />
                        </div>
                        <button
                          onClick={async () => {
                            const input = (removeInputBySlug[allianceKey] ?? '').trim()
                            if (!input) return
                            setRemovingPlayerSlug(allianceKey)
                            setRemovePlayerError(null)
                            const { ok, error } = await api.removeAllianceMember(
                              alliance.owner_account,
                              alliance.owner_server,
                              alliance.slug,
                              input
                            )
                            setRemovingPlayerSlug(null)
                            if (ok) {
                              setRemoveInputBySlug((prev) => ({ ...prev, [allianceKey]: '' }))
                              loadAlliances()
                            } else {
                              setRemovePlayerError(error ?? 'Failed to remove player')
                            }
                          }}
                          disabled={removingPlayerSlug !== null || !(removeInputBySlug[allianceKey] ?? '').trim()}
                          className="w-[130px] shrink-0 px-4 py-2.5 bg-red-600 hover:bg-red-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-all"
                        >
                          {removingPlayerSlug === allianceKey ? (
                            <i className="fas fa-spinner fa-spin mr-2"></i>
                          ) : (
                            <i className="fas fa-minus mr-2"></i>
                          )}
                          Remove
                        </button>
                      </div>

                      <div className="flex flex-wrap gap-3">
                        {[...alliance.players]
                          .sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))
                          .map((player) => (
                          <div
                            key={player.player_id}
                            className="flex items-center gap-3 px-4 py-2 bg-gray-700/50 rounded-lg border border-gray-600 hover:border-indigo-500/50 transition-all"
                          >
                            {player.avatar_image ? (
                              <img
                                src={player.avatar_image}
                                alt=""
                                className="w-10 h-10 rounded-full object-cover"
                              />
                            ) : (
                              <div className="w-10 h-10 rounded-full bg-indigo-600 flex items-center justify-center text-white font-semibold">
                                {player.name.charAt(0).toUpperCase()}
                              </div>
                            )}
                            <div>
                              <p className="font-medium text-white">{player.name}</p>
                              <p className="text-xs text-gray-400">
                                ID: {player.player_id}
                                {player.castle_level && ` · ${player.castle_level}`}
                              </p>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )})}
                </div>
              )}
            </div>
          )}

          {/* Giftcode Automation Tab */}
          {activeTab === 'giftcode-automation' && (
            <TabGiftcodeAutomation accountName={accountName ?? null} serverNumber={serverNumber} />
          )}

          {/* Swordland Tab */}
          {activeTab === 'swordland' && (
            <TabSwordland accountName={accountName ?? null} serverNumber={serverNumber} />
          )}

          {/* Tri Alliance Tab */}
          {activeTab === 'tri-alliance' && (
            <TabTriAlliance accountName={accountName ?? null} serverNumber={serverNumber} />
          )}

          {activeTab === 'manage-server-org' && (
            <TabManageServer accountName={accountName ?? null} serverNumber={serverNumber} />
          )}

          {activeTab === 'tyrant' && (
            <TabTyrant accountName={accountName ?? null} serverNumber={serverNumber} />
          )}

          {/* Statistics Tab */}
          {activeTab === 'stats' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-blue-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-chart-bar text-blue-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Statistics</h2>
                <p className="text-gray-400">View alliance and time slot statistics</p>
              </div>

              {statsLoading && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
                  <p className="text-xl text-gray-400">Loading statistics...</p>
                </div>
              )}

              {!statsLoading && statsError && (
                <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {statsError}
                </div>
              )}

              {!statsLoading && !statsError && stats && (
                <div className="space-y-8">
                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                      <i className="fas fa-users text-blue-400 mr-3"></i>Alliance Request Counts
                    </h2>
                    <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
                      {sortedAlliances.map(([alliance, allianceData]) => (
                        <div
                          key={alliance}
                          className="bg-gray-700/50 rounded-xl p-5 border-2 border-gray-600 hover:border-blue-500 hover:shadow-lg transition-all"
                        >
                          <h3 className="text-xl font-bold text-white mb-4">
                            {alliance || '(No Alliance)'}
                          </h3>
                          <div className="space-y-2">
                            <div className="flex justify-between items-center">
                              <span className="text-gray-300 flex items-center">
                                <i className="fas fa-hammer text-orange-400 mr-2"></i>Construction:
                              </span>
                              <strong className="text-orange-400 text-lg">
                                {allianceData.construction_requests}
                              </strong>
                            </div>
                            <div className="flex justify-between items-center">
                              <span className="text-gray-300 flex items-center">
                                <i className="fas fa-flask text-blue-400 mr-2"></i>Research:
                              </span>
                              <strong className="text-blue-400 text-lg">
                                {allianceData.research_requests}
                              </strong>
                            </div>
                            <div className="flex justify-between items-center">
                              <span className="text-gray-300 flex items-center">
                                <i className="fas fa-users text-green-400 mr-2"></i>Troops:
                              </span>
                              <strong className="text-green-400 text-lg">
                                {allianceData.troops_requests}
                              </strong>
                            </div>
                            <div className="flex justify-between items-center pt-3 border-t-2 border-gray-600 mt-3">
                              <span className="font-bold text-gray-200">Total:</span>
                              <strong className="text-blue-400 text-xl">{getTotal(allianceData)}</strong>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  {Object.keys(sortedConstructionTimeSlots).length > 0 && (
                    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                      <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                        <i className="fas fa-hammer text-orange-400 mr-3"></i>Construction Day Time
                        Slot Popularity
                      </h2>
                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                        {Object.entries(sortedConstructionTimeSlots).map(([time, timeData]) => (
                          <div
                            key={'construction-' + time}
                            className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-orange-500 hover:shadow-md transition-all text-center"
                          >
                            <div className="font-bold text-orange-400 text-sm mb-1">{time}</div>
                            <div className="text-xl font-bold text-white">{timeData.requests}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(sortedResearchTimeSlots).length > 0 && (
                    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                      <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                        <i className="fas fa-flask text-blue-400 mr-3"></i>Research Day Time Slot
                        Popularity
                      </h2>
                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                        {Object.entries(sortedResearchTimeSlots).map(([time, timeData]) => (
                          <div
                            key={'research-' + time}
                            className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-blue-500 hover:shadow-md transition-all text-center"
                          >
                            <div className="font-bold text-blue-400 text-sm mb-1">{time}</div>
                            <div className="text-xl font-bold text-white">{timeData.requests}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(sortedTroopsTimeSlots).length > 0 && (
                    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                      <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                        <i className="fas fa-users text-green-400 mr-3"></i>Troops Training Day Time
                        Slot Popularity
                      </h2>
                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                        {Object.entries(sortedTroopsTimeSlots).map(([time, timeData]) => (
                          <div
                            key={'troops-' + time}
                            className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-green-500 hover:shadow-md transition-all text-center"
                          >
                            <div className="font-bold text-green-400 text-sm mb-1">{time}</div>
                            <div className="text-xl font-bold text-white">{timeData.requests}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(sortedConstructionTimeSlots).length === 0 &&
                    sortedTimeSlots.length > 0 && (
                      <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                        <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                          <i className="fas fa-clock text-blue-400 mr-3"></i>Time Slot Popularity
                        </h2>
                        <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                          {sortedTimeSlots.map(([time, timeData]) => (
                            <div
                              key={time}
                              className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-blue-500 hover:shadow-md transition-all text-center"
                            >
                              <div className="font-bold text-blue-400 text-sm mb-1">{time}</div>
                              <div className="space-y-1 text-xs">
                                <div className="flex justify-between">
                                  <span className="text-orange-400">
                                    <i className="fas fa-hammer"></i>
                                  </span>
                                  <strong className="text-gray-200">
                                    {timeData.construction_requests}
                                  </strong>
                                </div>
                                <div className="flex justify-between">
                                  <span className="text-blue-400">
                                    <i className="fas fa-flask"></i>
                                  </span>
                                  <strong className="text-gray-200">
                                    {timeData.research_requests}
                                  </strong>
                                </div>
                                <div className="flex justify-between">
                                  <span className="text-green-400">
                                    <i className="fas fa-users"></i>
                                  </span>
                                  <strong className="text-gray-200">
                                    {timeData.troops_requests}
                                  </strong>
                                </div>
                              </div>
                              <div className="mt-1 pt-1 border-t border-gray-600 font-bold text-blue-400 text-xs">
                                {getTotal(timeData)}
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                </div>
              )}

              {!statsLoading && !statsError && !stats && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-chart-bar text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400">
                    No statistics available. Generate a schedule first.
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Create Form Tab */}
          {activeTab === 'create-form' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-purple-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-cog text-purple-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Create Form</h2>
                <p className="text-gray-400">
                  Configure alliances and schedule times, then create a form to get a shareable link
                </p>
              </div>

              {createdFormUrl && (
                <div className="mb-8 p-6 bg-green-900/50 border-l-4 border-green-500 rounded-lg">
                  <h3 className="text-xl font-bold text-green-200 mb-3">
                    <i className="fas fa-check-circle mr-2"></i>Form Created Successfully!
                  </h3>
                  <p className="text-gray-300 mb-3">Share this link with your players to fill out the form:</p>
                  <div className="flex items-center gap-2 bg-gray-800 p-3 rounded-lg border border-gray-700 mb-3">
                    <input
                      id="formUrlInput"
                      type="text"
                      value={createdFormUrl}
                      readOnly
                      className="flex-1 bg-transparent text-white font-mono text-sm outline-none"
                    />
                    <button
                      onClick={() => copyToClipboard(createdFormUrl, 'formUrlInput')}
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                    >
                      <i className="fas fa-copy mr-2"></i>Copy
                    </button>
                  </div>
                  <div className="flex gap-2 flex-wrap">
                    <a
                      href={createdFormUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all text-sm"
                    >
                      <i className="fas fa-external-link-alt mr-2"></i>Open Form
                    </a>
                    <a
                      href={createdFormUrl + '/stats'}
                      target="_blank"
                      rel="noreferrer"
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all text-sm"
                    >
                      <i className="fas fa-chart-bar mr-2"></i>View Statistics
                    </a>
                  </div>
                  {createdFormCode && (
                    <p className="text-sm text-gray-400 mt-3">
                      Form Code: <span className="font-mono text-green-300">{createdFormCode}</span>
                    </p>
                  )}
                </div>
              )}

              <div className="space-y-8">
                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-tag mr-2"></i>Form Name
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">
                    Give this form a name (e.g., &quot;Week 1&quot;, &quot;January 2025&quot;). Optional.
                  </p>
                  <input
                    value={config.form_name}
                    onChange={(e) => setConfig((c) => ({ ...c, form_name: e.target.value }))}
                    type="text"
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="Enter form name (optional)"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-crown mr-2"></i>Kingdom ID <span className="text-red-400">*</span>
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">
                    The kingdom ID used to validate applicants.
                  </p>
                  <input
                    value={config.kingdom_id}
                    onChange={(e) => setConfig((c) => ({ ...c, kingdom_id: e.target.value }))}
                    type="text"
                    required
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="e.g. 123 or 456"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-info-circle mr-2"></i>Introduction Text
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">Standard introduction text (fixed).</p>
                  <textarea
                    value={STANDARD_INTRO_TEXT}
                    readOnly
                    disabled
                    rows={15}
                    className="w-full px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-gray-400 cursor-not-allowed font-mono text-sm"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-user-headset mr-2"></i>Support Person&apos;s Name
                  </h3>
                  <input
                    value={config.support_person_name}
                    onChange={(e) => setConfig((c) => ({ ...c, support_person_name: e.target.value }))}
                    type="text"
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="e.g. [COB]Vor"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-users mr-2"></i>Alliances <span className="text-red-400">*</span>
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">Add or remove alliance names.</p>
                  <button
                    type="button"
                    onClick={() =>
                      setConfig((c) => ({
                        ...c,
                        include_non_of_above: !c.include_non_of_above,
                      }))
                    }
                    className={`w-full px-4 py-3 rounded-lg font-medium transition-all flex items-center justify-center gap-2 mb-4 ${
                      config.include_non_of_above
                        ? 'bg-green-600/30 border-2 border-green-500 text-green-200 hover:bg-green-600/50'
                        : 'bg-gray-700 border-2 border-gray-600 text-gray-400 hover:bg-gray-600'
                    }`}
                  >
                    <i
                      className={
                        config.include_non_of_above ? 'fas fa-check-circle' : 'fas fa-times-circle'
                      }
                    ></i>
                    {config.include_non_of_above
                      ? 'Include "Non of the above"'
                      : 'Exclude "Non of the above"'}
                  </button>
                  <div className="space-y-3">
                    {config.alliances.map((alliance, index) => (
                      <div key={index} className="flex items-center gap-2">
                        <input
                          value={alliance}
                          onChange={(e) =>
                            setConfig((c) => {
                              const next = [...c.alliances]
                              next[index] = e.target.value
                              return { ...c, alliances: next }
                            })
                          }
                          type="text"
                          className="flex-1 px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                          placeholder="Alliance name"
                        />
                        <button
                          onClick={() =>
                            setConfig((c) => ({
                              ...c,
                              alliances: c.alliances.filter((_, i) => i !== index),
                            }))
                          }
                          className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-all"
                        >
                          <i className="fas fa-trash"></i>
                        </button>
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        setConfig((c) => ({ ...c, alliances: [...c.alliances, ''] }))
                      }
                      className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                    >
                      <i className="fas fa-plus mr-2"></i>Add Alliance
                    </button>
                  </div>
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-amber-400 mb-4">
                    <i className="fas fa-coins mr-2"></i>Current Age of Your Kingdom <span className="text-red-400">*</span>
                  </h3>
                  <div className="space-y-2">
                    {[
                      {
                        value: 'pre_truegold',
                        label: 'Pre-truegold',
                        desc: 'Neither truegold, tempered truegold, nor truegold dust unlocked',
                      },
                      {
                        value: 'truegold_unlocked',
                        label: 'Truegold unlocked',
                        desc: 'Truegold unlocked but not tempered truegold',
                      },
                      {
                        value: 'war_academy_unlocked',
                        label: 'War academy unlocked',
                        desc: 'Truegold and truegold dust unlocked, but not tempered truegold',
                      },
                      {
                        value: 'tempered_truegold_unlocked',
                        label: 'Tempered truegold unlocked',
                        desc: 'Truegold, tempered truegold, and truegold dust unlocked',
                      },
                    ].map((opt) => (
                      <label
                        key={opt.value}
                        className={`flex items-start gap-3 p-3 rounded-lg cursor-pointer hover:bg-gray-600/50 transition-colors ${
                          config.construction_truegold_mode === opt.value
                            ? 'bg-amber-900/30 border border-amber-600/50'
                            : 'border border-transparent'
                        }`}
                      >
                        <input
                          type="radio"
                          checked={config.construction_truegold_mode === opt.value}
                          onChange={() =>
                            setConfig((c) => ({
                              ...c,
                              construction_truegold_mode: opt.value,
                            }))
                          }
                          className="mt-1"
                        />
                        <div>
                          <span className="font-semibold text-white">{opt.label}</span>
                          <p className="text-sm text-gray-400">{opt.desc}</p>
                        </div>
                      </label>
                    ))}
                  </div>
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-2">
                    <i className="fas fa-calendar-day mr-2"></i>Construction &amp; Research Days
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">
                    Choose which day slot is used for Construction and which for Research. Each option can only be used once. The two Friday options are paired (same day, different time windows).
                  </p>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                      <label className="block text-sm font-semibold text-orange-300 mb-2">Construction day</label>
                      <select
                        value={config.construction_day_slot}
                        onChange={(e) =>
                          setConfig((c) => ({
                            ...c,
                            construction_day_slot: e.target.value as BuildingResearchDaySlot,
                          }))
                        }
                        className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-orange-500 focus:ring-2 focus:ring-orange-500/50 outline-none"
                      >
                        <option value="monday" disabled={config.research_day_slot === 'monday'}>
                          Monday 00:00 – 24:00
                          {config.research_day_slot === 'monday' ? ' (used for Research)' : ''}
                        </option>
                        <option value="tuesday" disabled={config.research_day_slot === 'tuesday'}>
                          Tuesday 00:00 – 24:00
                          {config.research_day_slot === 'tuesday' ? ' (used for Research)' : ''}
                        </option>
                        <optgroup label="Friday">
                          <option
                            value="friday_full"
                            disabled={
                              config.research_day_slot === 'friday_full' ||
                              config.research_day_slot === 'friday_sat'
                            }
                          >
                            Friday 00:00 – 24:00
                            {config.research_day_slot === 'friday_full' ? ' (used for Research)' : ''}
                          </option>
                          <option
                            value="friday_sat"
                            disabled={
                              config.research_day_slot === 'friday_full' ||
                              config.research_day_slot === 'friday_sat'
                            }
                          >
                            Friday 10:00 – Saturday 10:00
                            {config.research_day_slot === 'friday_sat' ? ' (used for Research)' : ''}
                          </option>
                        </optgroup>
                      </select>
                    </div>
                    <div>
                      <label className="block text-sm font-semibold text-blue-300 mb-2">Research day</label>
                      <select
                        value={config.research_day_slot}
                        onChange={(e) =>
                          setConfig((c) => ({
                            ...c,
                            research_day_slot: e.target.value as BuildingResearchDaySlot,
                          }))
                        }
                        className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                      >
                        <option value="monday" disabled={config.construction_day_slot === 'monday'}>
                          Monday 00:00 – 24:00
                          {config.construction_day_slot === 'monday' ? ' (used for Construction)' : ''}
                        </option>
                        <option value="tuesday" disabled={config.construction_day_slot === 'tuesday'}>
                          Tuesday 00:00 – 24:00
                          {config.construction_day_slot === 'tuesday' ? ' (used for Construction)' : ''}
                        </option>
                        <optgroup label="Friday">
                          <option
                            value="friday_full"
                            disabled={
                              config.construction_day_slot === 'friday_full' ||
                              config.construction_day_slot === 'friday_sat'
                            }
                          >
                            Friday 00:00 – 24:00
                            {config.construction_day_slot === 'friday_full' ? ' (used for Construction)' : ''}
                          </option>
                          <option
                            value="friday_sat"
                            disabled={
                              config.construction_day_slot === 'friday_full' ||
                              config.construction_day_slot === 'friday_sat'
                            }
                          >
                            Friday 10:00 – Saturday 10:00
                            {config.construction_day_slot === 'friday_sat' ? ' (used for Construction)' : ''}
                          </option>
                        </optgroup>
                      </select>
                    </div>
                  </div>
                  {(config.construction_day_slot === config.research_day_slot ||
                    ((config.construction_day_slot === 'friday_full' ||
                      config.construction_day_slot === 'friday_sat') &&
                      (config.research_day_slot === 'friday_full' ||
                        config.research_day_slot === 'friday_sat'))) && (
                    <p className="text-sm text-amber-400 mt-2">
                      <i className="fas fa-exclamation-triangle mr-1"></i>Construction and Research must be on different
                      days. The two Friday options count as the same day.
                    </p>
                  )}
                </div>

                <button
                  onClick={() => handleCreateForm()}
                  disabled={creatingForm}
                  className="w-full bg-purple-600 hover:bg-purple-700 text-white px-6 py-3 rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <i className={`fas ${creatingForm ? 'fa-spinner fa-spin' : 'fa-plus-circle'} mr-2`}></i>
                  {creatingForm ? 'Creating Form...' : 'Create Form'}
                </button>

                {configStatus && (
                  <div
                    className={`p-4 rounded-lg ${
                      configStatus.type === 'success'
                        ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200'
                        : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'
                    }`}
                  >
                    <i
                      className={`fas ${
                        configStatus.type === 'success' ? 'fa-check-circle' : 'fa-times-circle'
                      } mr-2`}
                    ></i>
                    {configStatus.message}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Current Form Tab */}
          {activeTab === 'current-form' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-purple-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-file-alt text-purple-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Current Form</h2>
                <p className="text-gray-400">View and manage your current form</p>
              </div>

              {loadingCurrentForm && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
                  <p className="text-xl text-gray-400">Loading form information...</p>
                </div>
              )}

              {!loadingCurrentForm && currentForm && (
                <div className="space-y-6">
                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <div className="space-y-4">
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Form Name:</p>
                        <p className="text-2xl font-bold text-white">{currentForm.name}</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Form Code:</p>
                        <p className="text-lg font-mono text-purple-300">{currentForm.code}</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Created:</p>
                        <p className="text-gray-200">{formatDate(currentForm.created_at)}</p>
                      </div>
                      {(currentForm as { delete_date?: string }).delete_date && (
                        <div>
                          <p className="text-sm text-gray-300 mb-1">Auto-archives:</p>
                          <p className="text-amber-400">{(currentForm as { delete_date?: string }).delete_date}</p>
                        </div>
                      )}
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Responses:</p>
                        <p className="text-3xl font-bold text-green-400">
                          {currentForm.submissions_count ?? 0}
                        </p>
                      </div>
                    </div>
                  </div>

                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <h3 className="text-xl font-bold text-white mb-4">Form Link</h3>
                    <div className="flex items-center gap-2 bg-gray-700 p-3 rounded-lg border border-gray-600 mb-4">
                      <input
                        id="currentFormUrlInput"
                        type="text"
                        value={currentForm.url}
                        readOnly
                        className="flex-1 bg-transparent text-white font-mono text-sm outline-none"
                      />
                      <button
                        onClick={() => copyToClipboard(currentForm.url, 'currentFormUrlInput')}
                        className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all"
                      >
                        <i className="fas fa-copy mr-2"></i>Copy Link
                      </button>
                    </div>
                    <div className="flex gap-2 flex-wrap">
                      <a
                        href={currentForm.url}
                        target="_blank"
                        rel="noreferrer"
                        className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all"
                      >
                        <i className="fas fa-external-link-alt mr-2"></i>Open Form
                      </a>
                      <a
                        href={currentForm.url + '/stats'}
                        target="_blank"
                        rel="noreferrer"
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                      >
                        <i className="fas fa-chart-bar mr-2"></i>View Statistics
                      </a>
                    </div>
                  </div>

                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <div className="flex justify-between items-center mb-4">
                      <h3 className="text-xl font-bold text-white">
                        <i className="fas fa-table mr-2"></i>All Form Submissions
                      </h3>
                      <button
                        onClick={loadSubmissions}
                        disabled={loadingSubmissions}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <i
                          className={`fas ${
                            loadingSubmissions ? 'fa-spinner fa-spin' : 'fa-sync-alt'
                          } mr-2`}
                        ></i>
                        {loadingSubmissions ? 'Reloading...' : 'Reload'}
                      </button>
                    </div>
                    {loadingSubmissions && (
                      <div className="text-center py-8">
                        <i className="fas fa-spinner fa-spin text-2xl text-blue-400 mb-2"></i>
                        <p className="text-gray-400">Loading submissions...</p>
                      </div>
                    )}
                    {!loadingSubmissions && submissionsError && (
                      <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                        <i className="fas fa-exclamation-circle mr-2"></i>
                        {submissionsError}
                      </div>
                    )}
                    {!loadingSubmissions && !submissionsError && submissions && submissions.length > 0 && (
                      <div className="overflow-x-auto">
                        <table className="min-w-full text-left border-collapse" style={{ minWidth: 1600 }}>
                          <thead>
                            <tr className="border-b border-gray-700">
                              {SUBMISSION_HEADERS.map((header) => (
                                <th
                                  key={header}
                                  className={`px-4 py-3 text-gray-300 font-semibold bg-gray-700/50 whitespace-nowrap ${
                                    ['Troop times', 'Construction times', 'Research times'].includes(header)
                                      ? 'min-w-[300px]'
                                      : ''
                                  }`}
                                >
                                  {header}
                                </th>
                              ))}
                            </tr>
                          </thead>
                          <tbody>
                            {submissions.map((submission, index) => (
                              <tr
                                key={index}
                                className="border-b border-gray-700 hover:bg-gray-700/30 transition-colors"
                              >
                                {SUBMISSION_HEADERS.map((header) => (
                                  <td
                                    key={header}
                                    className={`px-4 py-3 text-gray-200 ${
                                      ['Troop times', 'Construction times', 'Research times'].includes(
                                        header
                                      )
                                        ? 'min-w-[300px]'
                                        : ''
                                    }`}
                                  >
                                    {getSubmissionValue(
                                      submission as Record<string, unknown>,
                                      header
                                    )}
                                  </td>
                                ))}
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    )}
                    {!loadingSubmissions && !submissionsError && (!submissions || submissions.length === 0) && (
                      <div className="text-center py-8 text-gray-400">
                        <i className="fas fa-inbox text-4xl mb-2"></i>
                        <p>No submissions yet</p>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {!loadingCurrentForm && !currentForm && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-file-alt text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400 mb-4">No current form found.</p>
                  <p className="text-gray-500">Create a new form using the &quot;Create Form&quot; tab, or reopen an archived form below.</p>
                </div>
              )}

              <div className="bg-gray-800 rounded-lg shadow-xl p-8 mb-6 border border-gray-700 mt-8">
                <h3 className="text-xl font-bold text-white mb-4">
                  <i className="fas fa-history mr-2"></i>Reopen Archived Form
                </h3>
                <p className="text-gray-400 mb-4">Forms are archived 2 weeks after creation. Reopen a previously archived form to use it again.</p>
                {loadingOldForms ? (
                  <div className="text-center py-4">
                    <i className="fas fa-spinner fa-spin text-2xl text-blue-400"></i>
                  </div>
                ) : oldForms.length === 0 ? (
                  <p className="text-gray-500">No archived forms.</p>
                ) : (
                  <div className="space-y-2">
                    {oldForms.map((f) => (
                      <div
                        key={f.archive_name}
                        className="flex items-center justify-between gap-4 p-4 bg-gray-700/50 rounded-lg border border-gray-600"
                      >
                        <div>
                          <p className="font-semibold text-white">{f.name}</p>
                          <p className="text-sm text-gray-400">
                            Code: {f.code} • Created: {formatDate(f.created_at)}
                            {f.delete_date && ` • Archived: ${f.delete_date}`}
                          </p>
                        </div>
                        <button
                          onClick={() => handleReopenForm(f.archive_name)}
                          disabled={reopeningForm}
                          className="px-4 py-2 bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white rounded-lg font-medium transition-all"
                        >
                          {reopeningForm ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-folder-open mr-2"></i>}
                          Reopen
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* CSV Operations Tab */}
          {activeTab === 'csv-operations' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-green-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-file-csv text-green-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">CSV Operations</h2>
                <p className="text-gray-400">Upload or download CSV files</p>
              </div>

              <div className="bg-gray-800 rounded-lg shadow-xl p-6 mb-6 border border-gray-700">
                <h3 className="text-xl font-bold text-white mb-4">
                  <i className="fas fa-download mr-2"></i>Download Current Form CSV
                </h3>
                <p className="text-gray-400 mb-4">
                  Download the CSV file containing all form submissions
                </p>
                <button
                  onClick={handleDownloadCSV}
                  disabled={downloadingCSV}
                  className="w-full bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <i
                    className={`fas ${downloadingCSV ? 'fa-spinner fa-spin' : 'fa-download'} mr-2`}
                  ></i>
                  {downloadingCSV ? 'Downloading...' : 'Download Current Form CSV'}
                </button>
              </div>

              <form onSubmit={handleUpload} className="space-y-6">
                <div>
                  <label className="block text-sm font-semibold text-gray-300 mb-2">
                    <i className="fas fa-file-csv mr-2"></i>Select CSV File
                  </label>
                  <input
                    type="file"
                    accept=".csv"
                    required
                    onChange={(e) => {
                      setSelectedFile(e.target.files?.[0] ?? null)
                      setUploadStatus(null)
                    }}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:bg-blue-600 file:text-white hover:file:bg-blue-700 file:cursor-pointer"
                  />
                  {selectedFile && (
                    <p className="mt-2 text-sm text-gray-400">
                      <i className="fas fa-file mr-2"></i>Selected: {selectedFile.name}
                    </p>
                  )}
                </div>
                <button
                  type="submit"
                  disabled={uploading || !selectedFile}
                  className="w-full bg-green-600 hover:bg-green-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <i className={`fas ${uploading ? 'fa-spinner fa-spin' : 'fa-cloud-upload-alt'} mr-2`}></i>
                  {uploading ? 'Uploading and processing...' : 'Upload & Generate Schedule'}
                </button>
              </form>

              {uploadStatus && (
                <div
                  className={`mt-4 p-4 rounded-lg ${
                    uploadStatus.type === 'success'
                      ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200'
                      : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'
                  }`}
                >
                  <i
                    className={`fas ${
                      uploadStatus.type === 'success' ? 'fa-check-circle' : 'fa-times-circle'
                    } mr-2`}
                  ></i>
                  {uploadStatus.message}
                </div>
              )}
            </div>
          )}

          {/* Generate Schedule Tab */}
          {activeTab === 'generate-schedule' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-green-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-calendar-alt text-green-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Generate Schedule</h2>
                <p className="text-gray-400">Generate schedules from form submissions</p>
              </div>

              {!loadingCurrentForm && !currentForm && (
                <div className="bg-yellow-900/50 border-l-4 border-yellow-500 text-yellow-200 p-4 rounded-lg mb-6">
                  <i className="fas fa-exclamation-triangle mr-2"></i>
                  No current form found. Please create a form first.
                </div>
              )}

              <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600 mb-6">
                <h3 className="text-xl font-bold text-white mb-4">
                  <i className="fas fa-lock mr-2"></i>Predetermined Slots
                </h3>
                <p className="text-sm text-gray-400 mb-4">
                  Pre-assign specific time slots to players. Loaded from current form configuration.
                </p>
                {!currentForm ? (
                  <div className="text-gray-400 text-center py-4">
                    No current form found. Please create a form first.
                  </div>
                ) : (
                  <div className="space-y-3">
                    {predeterminedSlots.map((slot, index) => (
                      <div
                        key={index}
                        className="bg-gray-800 rounded-lg p-4 border border-gray-600"
                      >
                        <div className="grid md:grid-cols-2 gap-3 mb-3">
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Day <span className="text-red-400">*</span>
                            </label>
                            <select
                              value={slot.day}
                              onChange={(e) =>
                                setPredeterminedSlots((prev) => {
                                  const next = [...prev]
                                  next[index] = { ...next[index], day: e.target.value }
                                  return next
                                })
                              }
                              className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                            >
                              <option value="construction">Construction Day</option>
                              <option value="research">Research Day</option>
                              <option value="troops">Troops Training Day</option>
                            </select>
                          </div>
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Time (HH:MM) <span className="text-red-400">*</span>
                            </label>
                            <input
                              type="time"
                              value={slot.time}
                              onChange={(e) =>
                                setPredeterminedSlots((prev) => {
                                  const next = [...prev]
                                  next[index] = { ...next[index], time: e.target.value }
                                  return next
                                })
                              }
                              className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                            />
                          </div>
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Player ID <span className="text-red-400">*</span>
                            </label>
                            <input
                              type="text"
                              value={slot.player_id ?? ''}
                              onBlur={() => lookupPlayer(index)}
                              onChange={(e) =>
                                setPredeterminedSlots((prev) => {
                                  const next = [...prev]
                                  next[index] = { ...next[index], player_id: e.target.value }
                                  return next
                                })
                              }
                              placeholder="Enter player ID"
                              className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                            />
                            {slot.lookingUp && (
                              <div className="text-xs text-gray-400 mt-1">
                                <i className="fas fa-spinner fa-spin mr-1"></i>Looking up...
                              </div>
                            )}
                            {slot.lookupError && (
                              <div className="text-xs text-red-400 mt-1">
                                <i className="fas fa-exclamation-triangle mr-1"></i>
                                {slot.lookupError}
                              </div>
                            )}
                          </div>
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Alliance
                            </label>
                            <input
                              type="text"
                              value={slot.alliance ?? ''}
                              readOnly
                              disabled
                              className="w-full px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-gray-400 cursor-not-allowed"
                              placeholder="Auto-filled from player ID"
                            />
                          </div>
                          <div className="md:col-span-2">
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Player Name
                            </label>
                            <input
                              type="text"
                              value={slot.name ?? ''}
                              readOnly
                              disabled
                              className="w-full px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-gray-400 cursor-not-allowed"
                              placeholder="Auto-filled from player ID"
                            />
                          </div>
                        </div>
                        <button
                          onClick={() =>
                            setPredeterminedSlots((prev) => prev.filter((_, i) => i !== index))
                          }
                          className="w-full px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-all"
                        >
                          <i className="fas fa-trash mr-2"></i>Remove Slot
                        </button>
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        setPredeterminedSlots((prev) => [
                          ...prev,
                          {
                            day: 'construction',
                            time: '00:00',
                            player_id: '',
                            alliance: '',
                            name: '',
                            lookingUp: false,
                            lookupError: null,
                          },
                        ])
                      }
                      className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                    >
                      <i className="fas fa-plus mr-2"></i>Add Predetermined Slot
                    </button>
                  </div>
                )}
              </div>

              <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                <h3 className="text-lg font-semibold text-white mb-3">Generate all days</h3>
                <div className="flex flex-col sm:flex-row gap-4 mb-6">
                  <button
                    onClick={() => handleGenerateSchedule(false)}
                    disabled={generatingSchedule}
                    className="flex-1 bg-green-600 hover:bg-green-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <i
                      className={`fas ${
                        generatingSchedule ? 'fa-spinner fa-spin' : 'fa-magic'
                      } mr-2`}
                    ></i>
                    {generatingSchedule ? 'Generating...' : 'Generate Schedule (Replace)'}
                  </button>
                  <button
                    onClick={() => handleGenerateSchedule(true)}
                    disabled={generatingSchedule}
                    className="flex-1 bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <i
                      className={`fas ${
                        generatingSchedule ? 'fa-spinner fa-spin' : 'fa-plus'
                      } mr-2`}
                    ></i>
                    {generatingSchedule ? 'Generating...' : 'Append to Schedule'}
                  </button>
                </div>
                <p className="mb-6 text-sm text-gray-400">
                  <strong>Replace:</strong> Build a new schedule from scratch.{' '}
                  <strong>Append:</strong> Fill only empty slots, keep current assignments.
                </p>

                <h3 className="text-lg font-semibold text-white mb-3">Generate single day</h3>
                <p className="text-sm text-gray-400 mb-4">
                  Replace or append to one day. Crossover slot (Construction last = Research slot 1) is handled correctly.
                </p>
                <div className="space-y-4">
                  {(Object.keys(SCHEDULE_DAYS) as ScheduleDayKey[]).map((key) => (
                    <div key={key} className="flex flex-wrap items-center gap-2">
                      <span className="text-sm font-medium text-gray-400 w-24 sm:w-28">
                        <i className={`${SCHEDULE_DAYS[key].icon} mr-1.5 text-gray-500`}></i>
                        {SCHEDULE_DAYS[key].name}:
                      </span>
                      <button
                        onClick={() => handleGenerateSchedule(false, key)}
                        disabled={generatingSchedule}
                        className="px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white rounded-lg text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {generatingSchedule ? <i className="fas fa-spinner fa-spin mr-1.5"></i> : null}
                        Replace
                      </button>
                      <button
                        onClick={() => handleGenerateSchedule(true, key)}
                        disabled={generatingSchedule}
                        className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed ${
                          key === 'construction'
                            ? 'bg-orange-600 hover:bg-orange-700 text-white'
                            : key === 'research'
                            ? 'bg-blue-600 hover:bg-blue-700 text-white'
                            : 'bg-green-600 hover:bg-green-700 text-white'
                        }`}
                      >
                        {generatingSchedule ? <i className="fas fa-spinner fa-spin mr-1.5"></i> : <i className="fas fa-plus mr-1.5"></i>}
                        Append
                      </button>
                    </div>
                  ))}
                </div>

                {scheduleGenStatus && (
                  <div
                    className={`mt-4 p-4 rounded-lg ${
                      scheduleGenStatus.type === 'success'
                        ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200'
                        : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'
                    }`}
                  >
                    <i
                      className={`fas ${
                        scheduleGenStatus.type === 'success' ? 'fa-check-circle' : 'fa-times-circle'
                      } mr-2`}
                    ></i>
                    {scheduleGenStatus.message}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
