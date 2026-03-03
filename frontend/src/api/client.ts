export async function apiFetch<T>(
  url: string,
  options?: RequestInit
): Promise<{ data?: T; error?: string; ok: boolean }> {
  try {
    const res = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      credentials: 'include',
    })
    const data = await res.json().catch(() => ({}))
    if (!res.ok) {
      return { ok: false, error: (data as { error?: string }).error || 'Request failed' }
    }
    return { ok: true, data: data as T }
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Network error' }
  }
}

export const api = {
  login: (accountName: string, password: string) =>
    apiFetch<{ success?: boolean; account_name?: string; server_number?: number }>('/api/login', {
      method: 'POST',
      body: JSON.stringify({ account_name: accountName, password }),
    }),

  createAccount: (data: {
    account_name: string
    server_number: number
    in_game_name: string
    password: string
    player_id?: string
  }) =>
    apiFetch<{ success: boolean; schedule_url?: string }>('/api/create-account', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  getSession: () =>
    apiFetch<{
      account_name?: string
      server_number?: number
      player_id?: string
      in_game_name?: string
      is_admin?: boolean
    }>('/api/session'),

  listAdminAccounts: () =>
    apiFetch<{ success?: boolean; accounts?: Array<{ account_name: string; server_number: number; in_game_name: string; admin: boolean }> }>(
      '/api/admin/accounts'
    ),

  setAdmin: (accountName: string, admin: boolean) =>
    apiFetch<{ success?: boolean; account_name?: string; admin?: boolean }>(
      `/api/admin/accounts/${encodeURIComponent(accountName)}/admin`,
      { method: 'POST', body: JSON.stringify({ admin }) }
    ),

  updateProfile: (data: {
    account_name?: string
    server_number?: number
    in_game_name?: string
  }) =>
    apiFetch<{ success?: boolean; account_name?: string; server_number?: number; in_game_name?: string }>(
      '/api/profile/update',
      { method: 'PUT', body: JSON.stringify(data) }
    ),

  kingshotLookup: (playerId: string) =>
    apiFetch<{
      success?: boolean
      in_game_name?: string
      server_number?: number
      player_id?: string
    }>('/api/profile/kingshot-lookup', {
      method: 'POST',
      body: JSON.stringify({ player_id: playerId }),
    }),


  logout: () => apiFetch('/api/logout', { method: 'POST' }),

  getServers: () => apiFetch<{ servers: Array<{ account_name: string; server_number: number }> }>('/api/servers'),

  getFormStats: (formCode: string) =>
    apiFetch<FormStats>(`/form/${formCode}/api/stats`),

  getFormConfig: (formCode: string) =>
    apiFetch<FormConfig>(`/form/${formCode}/api/config`),

  submitForm: (formCode: string, data: FormSubmission) =>
    apiFetch<{ success: boolean; error?: string }>(`/form/${formCode}/api/submit`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  getSchedule: (account: string, server: number, day: string) =>
    apiFetch<Schedule>(`/${account}/${server}/api/schedule/${day}`),

  updateScheduleSlot: (account: string, server: number, day: string, time: string, player: string) =>
    apiFetch<{ success?: boolean }>(`/${account}/${server}/api/schedule/${day}/slot`, {
      method: 'PUT',
      body: JSON.stringify({ time, player }),
    }),

  getStats: (account: string, server: number) =>
    apiFetch<AccountStats>(`/${account}/${server}/api/stats`),

  accountLogin: (account: string, server: number, password: string) =>
    apiFetch<{ success: boolean }>(`/${account}/${server}/api/login`, {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),

  uploadCsv: (account: string, server: number, file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    return fetch(`/${account}/${server}/api/upload`, {
      method: 'POST',
      body: formData,
      credentials: 'include',
    }).then(async (res) => {
      const data = await res.json().catch(() => ({}))
      return { ok: res.ok, data, error: (data as { error?: string }).error }
    })
  },

  createForm: (account: string, server: number, config: CreateFormRequest) =>
    apiFetch<{ success: boolean; url?: string; form_url?: string; code?: string; form_code?: string }>(
      `/${account}/${server}/api/form/create`,
      { method: 'POST', body: JSON.stringify(config) }
    ),

  getCurrentForm: (account: string, server: number) =>
    apiFetch<CurrentFormInfo>(`/${account}/${server}/api/form/current`),

  listOldForms: (account: string, server: number) =>
    apiFetch<{ old_forms?: Array<{ archive_name: string; code: string; name: string; created_at: string; delete_date?: string }> }>(
      `/${account}/${server}/api/form/old`
    ),

  reopenForm: (account: string, server: number, archiveName: string) =>
    apiFetch<{ success?: boolean; code?: string; url?: string }>(`/${account}/${server}/api/form/reopen`, {
      method: 'POST',
      body: JSON.stringify({ archive_name: archiveName }),
    }),

  clearSchedule: (account: string, server: number, day?: 'construction' | 'research' | 'troops') =>
    apiFetch<{ success?: boolean; message?: string }>(`/${account}/${server}/api/schedule/clear`, {
      method: 'POST',
      body: JSON.stringify(day ? { day } : {}),
    }),

  getFormSubmissions: (account: string, server: number) =>
    apiFetch<{ submissions?: Record<string, unknown>[] }>(`/${account}/${server}/api/form/submissions`),

  downloadFormCsv: (account: string, server: number) =>
    fetch(`/${account}/${server}/api/form/download-csv`, { credentials: 'include' }),

  updateFormConfig: (account: string, server: number, predeterminedSlots: PredeterminedSlot[]) =>
    apiFetch<{ success?: boolean }>(`/${account}/${server}/api/form/config`, {
      method: 'PUT',
      body: JSON.stringify({ predetermined_slots: predeterminedSlots }),
    }),

  getPlayerById: (account: string, server: number, playerId: string) =>
    apiFetch<{ name?: string; alliance?: string }>(`/${account}/${server}/api/form/player/${playerId}`),

  getPreviousFormConfig: (account: string, server: number) =>
    apiFetch<{ success?: boolean; config?: { alliances?: string[]; include_non_of_above?: boolean; [key: string]: unknown } }>(`/${account}/${server}/api/form/previous`),

  generateSchedule: (append: boolean, day?: 'construction' | 'research' | 'troops') =>
    apiFetch<{ success?: boolean; message?: string; error?: string }>('/api/generate-schedule', {
      method: 'POST',
      body: JSON.stringify({ append, ...(day && { day }) }),
    }),

  checkSubmission: (formCode: string, playerId: string) =>
    apiFetch<{ has_submitted: boolean }>(`/form/${formCode}/api/check-submission/${playerId}`),

  playerLookup: (formCode: string, playerId: string) =>
    apiFetch<PlayerCard>(`/form/${formCode}/api/player-lookup/${playerId}`),

  submitFeedback: (data: { type: 'bug' | 'feature' | 'general'; text: string }) =>
    apiFetch<{ success?: boolean }>('/api/feedback', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  listFeedback: () =>
    apiFetch<{ success?: boolean; feedback?: Array<{ id: string; type: string; text: string; created_at: string }> }>(
      '/api/admin/feedback'
    ),

  archiveFeedback: (id: string) =>
    apiFetch<{ success?: boolean }>(`/api/admin/feedback/${encodeURIComponent(id)}/archive`, {
      method: 'POST',
    }),
}

export interface AccountStats {
  alliance_counts?: Record<string, { construction_requests: number; research_requests: number; troops_requests: number }>
  time_slot_popularity?: Record<string, { construction_requests: number; research_requests: number; troops_requests: number }>
  construction_time_slot_popularity?: Record<string, { requests: number }>
  research_time_slot_popularity?: Record<string, { requests: number }>
  troops_time_slot_popularity?: Record<string, { requests: number }>
  construction_start_time?: string
  research_start_time?: string
  troops_start_time?: string
}

export interface CreateFormRequest {
  name?: string
  form_name?: string
  kingdom_id: string
  support_person_name?: string
  alliances: string[]
  include_non_of_above: boolean
  construction_truegold_mode: string
  construction_times: { start_time: string; end_time?: string }
  research_times: { start_time: string; end_time?: string }
  troops_times: { start_time: string; end_time?: string }
   // Logical day slots (e.g. 'monday', 'tuesday', 'friday_full', 'friday_sat') for display/translation
  construction_day_slot?: string
  research_day_slot?: string
  predetermined_slots?: PredeterminedSlot[]
  intro_text?: string
}

export interface PredeterminedSlot {
  day: string
  time: string
  player_id?: string
  alliance?: string
  name?: string
}

export interface CurrentFormInfo {
  code: string
  name: string
  url: string
  delete_date?: string
  created_at: string
  submissions_count?: number
}

export interface PlayerCard {
  player_id: string
  name: string
  castle_level?: number | string
  kingdom?: string
  avatar_image?: string
}

export interface FormStats {
  construction_time_slot_popularity?: Record<string, { requests: number }>
  research_time_slot_popularity?: Record<string, { requests: number }>
  troops_time_slot_popularity?: Record<string, { requests: number }>
  construction_start_time?: string
  research_start_time?: string
  troops_start_time?: string
}

export interface FormConfig {
  alliances?: string[]
  include_non_of_above?: boolean
  construction_truegold_mode?: string
  construction_times?: { start_time: string; end_time?: string | null }
  research_times?: { start_time: string; end_time?: string | null }
  troops_times?: { start_time: string; end_time?: string | null }
  construction_day_slot?: string | null
  research_day_slot?: string | null
  intro_text?: string
  support_person_name?: string
  kingdom_id?: string
  [key: string]: unknown
}

export interface FormSubmission {
  alliance: string
  custom_alliance?: string
  character_name: string
  player_id: string
  submission_type: string
  wants_construction: boolean
  construction_speedups?: number
  construction_truegold?: number
  construction_tempered_truegold?: number
  construction_time_slots: number[]
  wants_research: boolean
  research_speedups?: number
  research_truegold_dust?: number
  research_time_slots: number[]
  wants_troops: boolean
  troops_speedups?: number
  troops_time_slots: number[]
  additional_notes?: string
  suggestions?: string
}

export interface Schedule {
  day_name?: string
  appointments?: Array<{ time: string; player?: string; is_empty?: boolean }>
  [key: string]: unknown
}
