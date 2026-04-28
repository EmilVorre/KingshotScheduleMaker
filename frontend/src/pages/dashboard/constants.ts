export const STANDARD_INTRO_TEXT =
  'Fill out this form to apply for Chief Minister (CM) and Noble Advisor (NA) appointments.\n\nSchedule:\n- Construction Day (Monday) [CM]\n- Research Day (Tuesday) [CM]\n- Troops Training Day (Thursday) [NA]\n\nRequirements:\n\n- Form must be filled out in order to be considered for an appointment during SvS preparation week. \n- Form must be filled out by THE SUNDAY OF MATCHMAKING.\n- Form filled out after the deadline will be added to the "Late" submission wait list.\n- Rally leaders and rally leader substitutes may be given priority (if necessary).\n- Verification of items, speedups, and resources may be requested (eg. during situations where the score is very close in points and to make sure our state wins by ensuring appointments go to players who can maximize points).\n\n\nFor more information:\n- Contact form support: #140 [COB]Vor and /or the current Minister of Justice if you have questions on filling out this form or changes to your form submission!'

export const SUBMISSION_HEADERS = [
  'Timestamp',
  'Name',
  'Alliance',
  'Construction speedups',
  'Truegold',
  'want Construction?',
  'Construction times',
  'Research Speedups',
  'Truegold Dust',
  'want Research?',
  'Research times',
  'Troop Speedups',
  'Want troops?',
  'Troop times',
]

export const SCHEDULE_DAYS = {
  construction: {
    name: 'Construction Day',
    icon: 'fas fa-hammer',
    buttonClass: 'bg-orange-600 hover:bg-orange-700 text-white',
    ringClass: 'ring-4 ring-orange-400',
  },
  research: {
    name: 'Research Day',
    icon: 'fas fa-flask',
    buttonClass: 'bg-blue-600 hover:bg-blue-700 text-white',
    ringClass: 'ring-4 ring-blue-400',
  },
  troops: {
    name: 'Troops Training Day',
    icon: 'fas fa-users',
    buttonClass: 'bg-green-600 hover:bg-green-700 text-white',
    ringClass: 'ring-4 ring-green-400',
  },
} as const

export type ScheduleDayKey = keyof typeof SCHEDULE_DAYS

export type Tab =
  | 'profile'
  | 'schedule'
  | 'alliance-application'
  | 'alliance-organisation'
  | 'manage-server-org'
  | 'tyrant'
  | 'giftcode-automation'
  | 'swordland'
  | 'tri-alliance'
  | 'stats'
  | 'create-form'
  | 'current-form'
  | 'csv-operations'
  | 'generate-schedule'

export const TAB_KEYS: Tab[] = [
  'profile',
  'schedule',
  'alliance-application',
  'alliance-organisation',
  'manage-server-org',
  'tyrant',
  'giftcode-automation',
  'swordland',
  'tri-alliance',
  'stats',
  'create-form',
  'current-form',
  'csv-operations',
  'generate-schedule',
]

export const ALLIANCE_LOCKED_TABS: Tab[] = [
  'alliance-organisation',
  'giftcode-automation',
  'swordland',
  'tri-alliance',
]

/** Tyrant tab requires workspace membership; Manage server is reachable to create the first workspace. */
export const SERVER_ORG_LOCKED_TABS: Tab[] = ['tyrant']
