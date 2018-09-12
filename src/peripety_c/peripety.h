/*
 * Copyright (C) 2017 Red Hat, Inc.
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 2.1 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; If not, see <http://www.gnu.org/licenses/>.
 *
 * Author: Gris Ge <fge@redhat.com>
 */

#ifndef _PERIPETY_H_
#define _PERIPETY_H_

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

struct peripety_event;
struct peripety_event_iter;
struct peripety_error;
struct peripety_blk_info;

enum peripety_event_filter_type {
	PERIPETY_EVENT_FILTER_TYPE_WWID = 0,
	PERIPETY_EVENT_FILTER_TYPE_EVEN_TYPE = 1,
	PERIPETY_EVENT_FILTER_TYPE_SEVERITY = 2,
	// ^ Equal or higher severity will match.
	PERIPETY_EVENT_FILTER_TYPE_SUBSYSTEM = 3,
	PERIPETY_EVENT_FILTER_TYPE_SINCE = 4,
	PERIPETY_EVENT_FILTER_TYPE_EVENTID = 5,
};

enum peripety_severity {
	PERIPEYT_SEVEITY_EMERGENCY = 0,
	PERIPEYT_SEVEITY_ALERT = 1,
	PERIPEYT_SEVEITY_CTRITICAL = 2,
	PERIPEYT_SEVEITY_ERROR = 3,
	PERIPEYT_SEVEITY_WARNING = 4,
	PERIPEYT_SEVEITY_NOTICE = 5,
	PERIPEYT_SEVEITY_INFO = 6,
	PERIPEYT_SEVEITY_DEBUG = 7,
	PERIPEYT_SEVEITY_UNKNOWN = 255,
}

enum peripety_blk_type {
	PERIPETY_BLK_TYPE_UNKNOWN,
	PERIPETY_BLK_TYPE_OTHER,
	PERIPETY_BLK_TYPE_SCSI,
	PERIPETY_BLK_TYPE_DM,
	PERIPETY_BLK_TYPE_DMMULTIPATH,
	PERIPETY_BLK_TYPE_DMLVM,
	PERIPETY_BLK_TYPE_PARTITION,
}

#define PERIPETY_ERR_OK					0
#define PERIPETY_ERR_LOG_SEVERITY_PARSE_ERROR		1
#define PERIPETY_ERR_CONF_ERROR				2
#define PERIPETY_ERR_JSON_SERIALIZE_ERROR		3
#define PERIPETY_ERR_JSON_DESERIALIZE_ERROR		4
#define PERIPETY_ERR_NO_SUPPORT				5
#define PERIPETY_ERR_INTERNAL_BUG			6
#define PERIPETY_ERR_BLOCK_NO_EXISTS			7
#define PERIPETY_ERR_STORAGE_SUBSYSTEM_PARSE_ERROR	8
#define PERIPETY_ERR_INVALID_ARGUMENT			9
#define PERIPETY_ERR_LOG_ACCESS_ERROR			10


struct peripety_event_iter *
	peripety_event_iter_new(struct peripety_error **error);

void peripety_event_iter_free(struct peripety_event_iter *se_iter);

void peripety_error_free(struct peripety_error *error);

// Will reset the current position of iterator.
int peripety_event_iter_add_filter(struct peripety_event_iter *pe_iter,
				   enum peripety_event_filter_type *type,
				   const char *value,
				   struct peripety_error **error);

int peripety_event_get_next(struct peripety_event_iter *pe_iter,
			   struct peripety_event **pe,
			   struct peripety_error **error);

void peripety_event_free(struct peripety_event *pe);

const char *peripety_event_hostname_get(struct peripety_event *pe);
enum peripety_severity peripety_event_severity_get(struct peripety_event *pe);
const char *peripety_event_severity_str_get(struct peripety_event *pe);
const char *peripety_event_sub_system_get(struct peripety_event *pe);
const char *peripety_event_ts_get(struct peripety_event *pe);
const char *peripety_event_id_get(struct peripety_event *pe);
const char *peripety_event_type_get(struct peripety_event *pe);
struct peripety_blk_info *peripety_event_blk_info_get(struct peripety_event *pe);
const char *peripety_event_msg_get(struct peripety_event *pe);
const char *peripety_event_raw_msg_get(struct peripety_event *pe);

const char *peripety_blk_info_wwid_get(struct peripety_blk_info *bi);
enum peripety_blk_type peripety_blk_info_type_get(struct peripety_blk_info *bi);
const char *peripety_blk_info_type_str_get(struct peripety_blk_info *bi);
const char *peripety_blk_info_pref_blk_path_get(struct peripety_blk_info *bi);
const char *peripety_blk_info_blk_path_get(struct peripety_blk_info *bi);
const char *peripety_blk_info_uuid_get(struct peripety_blk_info *bi);
const char *peripety_blk_info_mount_point_get(struct peripety_blk_info *bi);
const char *peripety_blk_info_trans_id_get(struct peripety_blk_info *bi);
void peripety_blk_info_owners_get(struct peripety_blk_info *bi,
				  struct peripety_blk_info ***owners_bis,
				  uint32_t *owner_count);
const char *peripety_error_msg_get(struct peripety_error *error);
int peripety_error_code_get(struct peripety_error *error);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* End of _PERIPETY_H_ */
