// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

#include <stdint.h>

struct c_callbacks {
    void *cb;
    void *data;
};

struct c_interface {
    const char *type_;
    uint32_t version;
    struct c_callbacks cb;
};
