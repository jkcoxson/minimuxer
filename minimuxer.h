// Jackson Coxson

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


/**
 * Starts the muxer and heartbeat client
 * # Arguments
 * Pairing file as a list of chars and the length
 * # Safety
 * Don't be stupid
 */
int minimuxer_c_start(char *pairing_file, unsigned int len);
